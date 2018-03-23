#![deny(warnings)]
extern crate bytes;
extern crate futures;
extern crate hyper;
extern crate hyper_gzip;
extern crate pretty_env_logger;
extern crate tokio;

use bytes::Bytes;

use futures::Future;
use futures::future::{FutureResult, lazy};

use hyper::{Body, Method, Request, Response, StatusCode};
use hyper::server::{Http, Service};
use hyper_gzip::server::Gzip;

static INDEX: &'static [u8] = b"Hello gzipped world!";

struct Echo;

impl Service for Echo {
    type Request = Request<Body>;
    type Response = Response<Bytes>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        futures::future::ok(match (req.method(), req.uri().path()) {
            (&Method::GET, "/")  => {
                Response::new(INDEX.into())
            },
            _ => {
                let mut res = Response::new(Bytes::new());
                *res.status_mut() = StatusCode::NOT_FOUND;
                res
            }
        })
    }

}


fn main() {
    pretty_env_logger::init();
    let addr = "127.0.0.1:1337".parse().unwrap();

    tokio::run(lazy(move || {
        let server = Http::new().bind(&addr, || Ok(Gzip::new(Echo))).unwrap();
        println!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
        server.run().map_err(|err| eprintln!("Server error {}", err))
    }));
}
