use std::borrow::Cow;
use std::ops::Deref;
use std::sync::Arc;

use arrayvec::ArrayVec;
use color_eyre::eyre::ContextCompat;
use fastwebsockets::CloseCode;
use fastwebsockets::FragmentCollector;
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::WebSocketError;
use fastwebsockets::WebSocketRead;
use fastwebsockets::upgrade;
use fastwebsockets::upgrade::UpgradeFut;
use futures_util::FutureExt;
use futures_util::Stream;
use futures_util::StreamExt;
use futures_util::TryFutureExt;
use futures_util::future;
use futures_util::future::BoxFuture;
use futures_util::stream;
use http_body_util::Empty;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::Service;
use tokio::io::AsyncRead;
use tokio::net::TcpListener;
use tracing_subscriber::fmt::format::FmtSpan;

fn get_room(req: &Request<Incoming>) -> Option<Cow<'_, str>> {
    let (_, room) = req
        .uri()
        .query()
        .and_then(|q| url::form_urlencoded::parse(q.as_bytes()).find(|(key, _)| key == "room"))?;
    Some(room)
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap();

    rt.block_on(async {
        let listener = TcpListener::bind("0.0.0.0:8080").await?;
        tracing::info!("Listening on 0.0.0.0:8080");
        let mut names = names::Generator::default();
        let tx = tokio::sync::broadcast::channel(16).0;
        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(
                http1::Builder::new()
                    .serve_connection(
                        hyper_util::rt::TokioIo::new(stream),
                        Svc {
                            broadcast: tx.clone(),
                            possible_room_name: names.next().unwrap(),
                        },
                    )
                    .with_upgrades()
                    .inspect_err(|err| println!("An error occurred: {err:?}")),
            );
        }
    })
}

#[derive(Clone)]
struct Svc {
    possible_room_name: String,
    // don't want to use a HashMap to avoid cleanup after disconnection
    broadcast: tokio::sync::broadcast::Sender<Arc<Guest>>,
}

impl Service<Request<Incoming>> for Svc {
    type Response = Response<Empty<Bytes>>;
    type Error = WebSocketError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        let (response, fut) = match upgrade::upgrade(&mut req) {
            Ok(a) => a,
            Err(err) => return future::err(err),
        };

        if let Some(room) = get_room(&req) {
            if self
                .broadcast
                .send(Arc::new(Guest {
                    fut: std::sync::Mutex::new(Some(fut)),
                    room: room.to_string(),
                }))
                .is_err()
            {
                return future::ok(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Empty::new())
                        .unwrap(),
                );
            }
        } else {
            let rx = self.broadcast.subscribe();
            let room = self.possible_room_name.clone();
            tokio::task::spawn(async move {
                if let Err(e) = host(room, fut, rx).await {
                    eprintln!("Error in websocket connection: {}", e);
                }
            });
        }

        future::ok(response)
    }
}

struct Guest {
    room: String,
    // the tasks won't wait a lot so we use the std Mutex
    fut: std::sync::Mutex<Option<UpgradeFut>>,
}

async fn host(
    room: String,
    fut: UpgradeFut,
    mut broadcast_rx: tokio::sync::broadcast::Receiver<Arc<Guest>>,
) -> color_eyre::Result<()> {
    let mut host = fut.await?;
    // it seems there are problems with auto messages
    // https://github.com/denoland/fastwebsockets/issues/87
    host.set_auto_close(false);
    host.set_auto_pong(false);
    let (host_rx, mut host_tx) = host.split(tokio::io::split);

    let mut host_messages = std::pin::pin!(bounded_msg_stream(host_rx));

    let guest: UpgradeFut = loop {
        futures_util::select! {
            frame = host_messages.next().fuse() => {
                let frame = frame.unwrap()?;

                match frame.opcode {
                    BoundedOpcode::Close => {
                        host_tx.write_frame(Frame::close(CloseCode::Normal.into(), &[])).await?;
                    },
                    BoundedOpcode::Ping => host_tx.write_frame(Frame::pong(fastwebsockets::Payload::Borrowed(&frame.payload))).await?,
                    _ => {}
                }
            },
            lol = broadcast_rx.recv().fuse() => {
                match lol {
                    Ok(guest) if room == guest.room => {
                            break guest.fut.try_lock().ok().and_then(|mut guard|guard.take()).context("Room name collision")?;
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return Err(color_eyre::Report::new(tokio::sync::broadcast::error::RecvError::Closed)),
                    // if it's lagging it's strange but whatever there is a timeout
                    _ => {}
                }
            }
        };
    };

    drop(broadcast_rx);
    drop(room);

    let mut guest = guest.await?;
    guest.set_auto_close(false);
    guest.set_auto_pong(false);
    let (guest_rx, mut guest_tx) = guest.split(tokio::io::split);

    let mut guest_messages = std::pin::pin!(bounded_msg_stream(guest_rx));

    loop {
        futures_util::select! {
            frame = host_messages.next().fuse() => {
                let frame = frame.unwrap()?;
            }
            frame = guest_messages.next().fuse() => {
                let frame = frame.unwrap()?;
            }
        }
    }
}

// Unfold::next is cancel safe. Isn't it beautiful
fn bounded_msg_stream<T: Unpin + AsyncRead + 'static + Send>(
    read: WebSocketRead<T>,
) -> impl Stream<Item = color_eyre::Result<BoundedFrame>> {
    stream::try_unfold(
        (read, BoundedFragments::default()),
        |(mut read, mut fragments)| async {
            let mut send_fn = |_| {
                unreachable!();
            };

            loop {
                let frame = read
                    .read_frame::<future::Ready<_>, WebSocketError>(&mut send_fn)
                    .await?;
                let mut payload = ArrayVec::new();
                payload.try_extend_from_slice(&frame.payload)?;
                if let Some(frame) = fragments.accumulate(BoundedFrame {
                    fin: frame.fin,
                    opcode: match frame.opcode {
                        OpCode::Continuation => BoundedOpcode::Continuation,
                        OpCode::Text => return Err(color_eyre::Report::msg("No text")),
                        OpCode::Binary => BoundedOpcode::Binary,
                        OpCode::Close => BoundedOpcode::Close,
                        OpCode::Ping => BoundedOpcode::Ping,
                        OpCode::Pong => BoundedOpcode::Pong,
                    },
                    payload,
                })? {
                    return Ok(Some((frame, (read, fragments))));
                }
            }
        },
    )
}

#[derive(Clone, Copy)]
pub enum BoundedOpcode {
    Continuation = 0x0,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

pub struct BoundedFrame {
    pub fin: bool,
    pub opcode: BoundedOpcode,
    pub payload: ArrayVec<u8, 4>,
}

pub struct BoundedFragment {
    opcode: BoundedOpcode,
    payload: ArrayVec<u8, 4>,
}

#[derive(Default)]
struct BoundedFragments {
    fragments: Option<BoundedFragment>,
}

impl BoundedFragments {
    pub fn accumulate(&mut self, frame: BoundedFrame) -> color_eyre::Result<Option<BoundedFrame>> {
        match frame.opcode {
            BoundedOpcode::Binary => {
                if !frame.fin {
                    self.fragments = Some(BoundedFragment {
                        payload: frame.payload,
                        opcode: frame.opcode,
                    });
                    return Ok(None);
                }

                if self.fragments.is_some() {
                    return Err(WebSocketError::InvalidFragment.into());
                }

                return Ok(Some(frame));
            }
            BoundedOpcode::Continuation => match self.fragments.as_mut() {
                None => {
                    return Err(WebSocketError::InvalidContinuationFrame.into());
                }
                Some(BoundedFragment { opcode, payload }) => {
                    payload.try_extend_from_slice(&frame.payload)?;
                    if frame.fin {
                        let payload = payload.take();
                        let opcode = *opcode;
                        self.fragments = None;
                        return Ok(Some(BoundedFrame {
                            fin: true,
                            opcode,
                            payload,
                        }));
                    }
                }
            },
            _ => return Ok(Some(frame)),
        }

        Ok(None)
    }
}
