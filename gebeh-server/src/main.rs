use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use arrayvec::ArrayVec;
use color_eyre::eyre::ContextCompat;
use fastwebsockets::CloseCode;
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::Payload;
use fastwebsockets::WebSocket;
use fastwebsockets::WebSocketError;
use fastwebsockets::WebSocketRead;
use fastwebsockets::WebSocketWrite;
use fastwebsockets::upgrade::UpgradeFut;
use futures_util::FutureExt;
use futures_util::Stream;
use futures_util::StreamExt;
use futures_util::TryFutureExt;
use futures_util::future;
use futures_util::stream;
use hyper::Request;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper_util::service::TowerToHyperService;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpListener;
use tokio::sync::broadcast::error::RecvError;
use tower::ServiceBuilder;
use tower::ServiceExt;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::instrument;
use tracing_subscriber::fmt::format::FmtSpan;

mod service;

fn get_room<T>(req: &Request<T>) -> Option<Cow<'_, str>> {
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
        .enable_time()
        .build()
        .unwrap();

    let names: Rc<RefCell<names::Generator>> = Default::default();
    let tx = tokio::sync::broadcast::channel(16).0;

    let upgrade_service = tower::service_fn(move |req| {
        let tx = tx.clone();
        let names = names.clone();
        service::upgrade(req, tx, names)
    });

    let assets_path = &*std::env::args()
        .nth(1)
        .context("Pease provide assets dir path in arguments")?
        .leak();

    let service = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .service(tower::service_fn(move |req| {
            let upgrade_service = upgrade_service.clone();
            service::route(
                req,
                "/ws",
                upgrade_service,
                ServiceExt::<Request<Incoming>>::map_err(
                    ServeDir::new(assets_path),
                    color_eyre::Report::new,
                ),
            )
        }));

    let service = TowerToHyperService::new(service);

    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, async {
        let listener = TcpListener::bind("0.0.0.0:8080").await?;
        tracing::info!("Listening on 0.0.0.0:8080");
        loop {
            let (stream, _) = listener.accept().await?;
            tokio::task::spawn_local(
                http1::Builder::new()
                    .serve_connection(hyper_util::rt::TokioIo::new(stream), service.clone())
                    .with_upgrades()
                    .inspect_err(|err| println!("An error occurred: {err:?}")),
            );
        }
    })
}

struct Guest {
    room: String,
    // the tasks won't wait a lot so we use the std Mutex
    fut: std::sync::Mutex<Option<UpgradeFut>>,
}

fn configure_ws<T>(ws: &mut WebSocket<T>) {
    // it seems there are problems with auto messages
    // https://github.com/denoland/fastwebsockets/issues/87
    ws.set_auto_close(false);
    ws.set_auto_pong(false);
    ws.set_max_message_size(64);
}

const TIMEOUT_GUEST_WAIT: Duration = Duration::from_mins(1);
const TIMEOUT_WS: Duration = Duration::from_secs(10);
const PING_PERIOD: Duration = Duration::from_secs(2);

#[instrument(skip(fut, broadcast_rx))]
async fn host(
    room: &str,
    fut: UpgradeFut,
    mut broadcast_rx: tokio::sync::broadcast::Receiver<Arc<Guest>>,
) -> color_eyre::Result<()> {
    let mut host = tokio::time::timeout(TIMEOUT_WS, fut).await??;
    configure_ws(&mut host);

    let (host_rx, mut host_tx) = host.split(tokio::io::split);

    let mut host_messages = std::pin::pin!(bounded_msg_stream(host_rx));

    tokio::time::timeout(
        TIMEOUT_WS,
        host_tx.write_frame(Frame::text(Payload::Borrowed(room.as_bytes()))),
    )
    .await??;

    let mut interval = tokio::time::interval(PING_PERIOD);

    let wait_guest_task = async {
        loop {
            futures_util::select! {
                _ = interval.tick().fuse() => {
                    handle_tick_single(&mut host_tx).await?;
                }
                frame = host_messages.next().fuse() => {
                    handle_frame_before_guest(frame.unwrap()?, &mut host_tx).await?;
                },
                guest = broadcast_rx.recv().fuse() => {
                    if let Some(guest) = handle_guest_broadcast(room, guest)? {
                        return Result::<_, color_eyre::Report>::Ok(guest);
                    }
                }
            };
        }
    };

    let guest = tokio::time::timeout(TIMEOUT_GUEST_WAIT, wait_guest_task).await??;

    tracing::info!("Guest is connected!");

    drop(broadcast_rx);

    let mut guest = tokio::time::timeout(TIMEOUT_GUEST_WAIT, guest).await??;
    configure_ws(&mut guest);

    let (guest_rx, mut guest_tx) = guest.split(tokio::io::split);

    let mut guest_messages = std::pin::pin!(bounded_msg_stream(guest_rx));

    // empty message to tell the host that the guest is connected
    tokio::time::timeout(
        TIMEOUT_WS,
        host_tx.write_frame(Frame::binary(fastwebsockets::Payload::Borrowed(&[]))),
    )
    .await??;

    // have to make the futures outside select to not cancel the timeout
    let mut host_message_timeout = tokio::time::timeout(TIMEOUT_WS, host_messages.next())
        .boxed()
        .fuse();

    let mut guest_message_timeout = tokio::time::timeout(TIMEOUT_WS, guest_messages.next())
        .boxed()
        .fuse();

    loop {
        futures_util::select! {
            _ = interval.tick().fuse() => {
                handle_tick(&mut host_tx, &mut guest_tx).await?;
            }
            frame = host_message_timeout => {
                handle_frame(frame?.unwrap()?, &mut host_tx, &mut guest_tx).await?;
                drop(host_message_timeout);
                host_message_timeout = tokio::time::timeout(TIMEOUT_WS, host_messages.next())
                    .boxed()
                    .fuse();
            }
            frame = guest_message_timeout => {
                handle_frame(frame?.unwrap()?, &mut guest_tx, &mut host_tx).await?;
                drop(guest_message_timeout);
                guest_message_timeout = tokio::time::timeout(TIMEOUT_WS, guest_messages.next())
                    .boxed()
                    .fuse();
            }
        }
    }
}

async fn handle_tick_single<T: Unpin + AsyncWrite>(
    tx0: &mut WebSocketWrite<T>,
) -> color_eyre::Result<()> {
    tokio::time::timeout(
        TIMEOUT_WS,
        tx0.write_frame(Frame::new(true, OpCode::Ping, None, Payload::Borrowed(&[]))),
    )
    .await??;

    Ok(())
}

async fn handle_tick<T: Unpin + AsyncWrite, U: Unpin + AsyncWrite>(
    tx0: &mut WebSocketWrite<T>,
    tx1: &mut WebSocketWrite<U>,
) -> color_eyre::Result<()> {
    tokio::time::timeout(
        TIMEOUT_WS,
        future::try_join(
            tx0.write_frame(Frame::new(true, OpCode::Ping, None, Payload::Borrowed(&[]))),
            tx1.write_frame(Frame::new(true, OpCode::Ping, None, Payload::Borrowed(&[]))),
        ),
    )
    .await??;

    Ok(())
}

async fn handle_frame_before_guest<T: Unpin + AsyncWrite>(
    frame: BoundedFrame,
    tx: &mut WebSocketWrite<T>,
) -> color_eyre::Result<()> {
    match frame.opcode {
        BoundedOpcode::Close => {
            tx.write_frame(Frame::close(CloseCode::Normal.into(), &[]))
                .await?;
            return Err(color_eyre::Report::msg("Host connection closed"));
        }
        BoundedOpcode::Ping => {
            tx.write_frame(Frame::pong(fastwebsockets::Payload::Borrowed(
                &frame.payload,
            )))
            .await?
        }
        _ => {}
    }

    Ok(())
}

fn handle_guest_broadcast(
    room: &str,
    res: Result<Arc<Guest>, RecvError>,
) -> color_eyre::Result<Option<UpgradeFut>> {
    match res {
        Ok(guest) if room == guest.room => {
            return Ok(Some(
                guest
                    .fut
                    .try_lock()
                    .ok()
                    .and_then(|mut guard| guard.take())
                    .context("Room name collision")?,
            ));
        }
        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
            return Err(color_eyre::Report::new(
                tokio::sync::broadcast::error::RecvError::Closed,
            ));
        }
        // if it's lagging it's strange but whatever there is a timeout
        _ => {}
    }

    Ok(None)
}

async fn handle_frame<T: Unpin + AsyncWrite, U: Unpin + AsyncWrite>(
    frame: BoundedFrame,
    current_tx: &mut WebSocketWrite<T>,
    other_tx: &mut WebSocketWrite<U>,
) -> color_eyre::Result<()> {
    match frame.opcode {
        BoundedOpcode::Close => {
            let close_task = future::try_join(
                current_tx.write_frame(Frame::close(CloseCode::Normal.into(), &[])),
                other_tx.write_frame(Frame::close(CloseCode::Away.into(), &[])),
            );
            tokio::time::timeout(TIMEOUT_WS, close_task).await??;
            return Err(color_eyre::Report::msg("Host connection closed"));
        }
        BoundedOpcode::Ping => {
            tokio::time::timeout(
                TIMEOUT_WS,
                current_tx.write_frame(Frame::pong(fastwebsockets::Payload::Borrowed(
                    &frame.payload,
                ))),
            )
            .await??
        }
        BoundedOpcode::Binary => {
            tokio::time::timeout(
                TIMEOUT_WS,
                other_tx.write_frame(Frame::binary(fastwebsockets::Payload::Borrowed(&[frame
                    .payload
                    .first()
                    .copied()
                    .context(color_eyre::Report::msg("Invalid message from host"))?]))),
            )
            .await??
        }
        _ => {}
    }

    Ok(())
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
