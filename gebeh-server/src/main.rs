use std::borrow::Cow;
use std::sync::Arc;

use color_eyre::eyre::ContextCompat;
use fastwebsockets::Frame;
use fastwebsockets::OpCode;
use fastwebsockets::WebSocketError;
use fastwebsockets::upgrade;
use fastwebsockets::upgrade::UpgradeFut;
use futures_util::FutureExt;
use futures_util::TryFutureExt;
use futures_util::future;
use http_body_util::Empty;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::Service;
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
    let (host_rx, mut host_tx) = fut.await?.split(tokio::io::split);
    let mut host_rx = fastwebsockets::FragmentCollectorRead::new(host_rx);

    let mut unreachable_fn = |_| {
        unreachable!();
    };

    let mut read_frame_fut = host_rx
        .read_frame::<future::Ready<_>, _>(&mut unreachable_fn)
        .boxed()
        .fuse();

    loop {
        futures_util::select! {
            frame = read_frame_fut => {
                let frame = frame?;

                match frame.opcode {
                    OpCode::Close => break,
                    OpCode::Ping => host_tx.write_frame(Frame::pong(frame.payload)).await?,
                    _ => {}
                }

                drop(read_frame_fut);
                read_frame_fut = host_rx
                    .read_frame::<_, WebSocketError>(&mut unreachable_fn)
                    .boxed().fuse();
            },
            lol = broadcast_rx.recv().fuse() => {
                match lol {
                    Ok(guest) if room == guest.room => {
                            // if err then collision in room names
                            let guest = guest.fut.try_lock().ok().and_then(|mut guard|guard.take()).context("Room name collision")?;
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return Err(color_eyre::Report::new(tokio::sync::broadcast::error::RecvError::Closed)),
                    // if it's lagging it's strange but whatever there is a timeout
                    _ => {}
                }
            }
        };
    }

    Ok(())
}
