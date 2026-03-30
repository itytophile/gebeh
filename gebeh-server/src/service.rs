use std::{cell::RefCell, rc::Rc, sync::Arc};

use fastwebsockets::upgrade;
use futures_util::future;
use http_body_util::combinators::UnsyncBoxBody;
use hyper::{Request, Response, StatusCode, body::Bytes};

use crate::{Guest, get_room, host};

pub fn upgrade<T>(
    mut req: Request<T>,
    tx: tokio::sync::broadcast::Sender<Arc<Guest>>,
    names: Rc<RefCell<names::Generator<'static>>>,
) -> impl Future<Output = color_eyre::Result<Response<UnsyncBoxBody<Bytes, color_eyre::Report>>>> {
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
                    .body(UnsyncBoxBody::default())
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

    future::ok(response.map(|_| Default::default()))
}

pub async fn route<Req, Err>(
    req: Request<Req>,
    route: &'static str,
    mut service: impl tower::Service<
        Request<Req>,
        Response = Response<UnsyncBoxBody<Bytes, color_eyre::Report>>,
        Error = Err,
    >,
    mut fallback: impl tower::Service<
        Request<Req>,
        Response = Response<UnsyncBoxBody<Bytes, color_eyre::Report>>,
        Error = Err,
    >,
) -> Result<Response<UnsyncBoxBody<Bytes, color_eyre::Report>>, Err> {
    if req.uri().path() == route {
        service.call(req).await
    } else {
        fallback.call(req).await
    }
}
