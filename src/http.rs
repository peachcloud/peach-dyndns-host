use futures::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};

fn handle_request(_req: Request<Body>) -> Response<Body> {
    Response::new(Body::from("Hello, World!"))
}

pub async fn server() {
    let address = ([127, 0, 0, 1], 3000).into();

    let handle_connection = || service_fn_ok(handle_request);

    info!("HTTP server listening for TCP on {:?}", address);

    let server = Server::bind(&address)
        .serve(handle_connection);
}
