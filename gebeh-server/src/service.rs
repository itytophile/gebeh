use std::{cell::RefCell, error::Error, rc::Rc, sync::Arc};

use fastwebsockets::upgrade;
use futures_util::future;
use http_body_util::Empty;
use hyper::{Request, Response, StatusCode, body::Bytes};

use crate::{Guest, get_room, host};

pub fn upgrade<T>(
    mut req: Request<T>,
    tx: tokio::sync::broadcast::Sender<Arc<Guest>>,
    names: Rc<RefCell<names::Generator<'static>>>,
) -> impl Future<Output = color_eyre::Result<Response<Empty<Bytes>>>> {
    let (response, fut) = match upgrade::upgrade(&mut req) {
        Ok(a) => a,
        Err(err) => return future::err(color_eyre::Report::new(err)),
    };

    if let Some(room) = get_room(&req) {
        if tx
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
        let rx = tx.subscribe();
        let room = names.borrow_mut().next().unwrap();
        tokio::task::spawn(async move {
            if let Err(err) = host(&room, fut, rx).await {
                tracing::warn!("Error in room {room}: {err}");
            }
        });
    }

    future::ok(response)
}

pub async fn route<T, U>(
    req: Request<T>,
    route: &'static str,
    mut service: impl tower::Service<
        Request<T>,
        Response = U,
        Error = impl Error + Send + Sync + 'static,
    >,
    mut fallback: impl tower::Service<
        Request<T>,
        Response = U,
        Error = impl Error + Send + Sync + 'static,
    >,
) -> color_eyre::Result<U> {
    if req.uri().path() == route {
        service.call(req).await.map_err(color_eyre::Report::new)
    } else {
        fallback.call(req).await.map_err(color_eyre::Report::new)
    }
}
