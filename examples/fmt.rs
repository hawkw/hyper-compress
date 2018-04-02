// #![deny(warnings)]
extern crate futures;
extern crate hyper;
extern crate hyper_compress;
extern crate pretty_env_logger;
extern crate tokio_core;
extern crate tokio_io;
#[macro_use]
extern crate log;
extern crate flate2;

use futures::{Future, Stream};
use futures::future::{self, FutureResult};
use hyper::{Body, Method, Response, StatusCode};
use hyper::server::{Http, Service};
use hyper_compress::server::{GzWriterService, GzWriterRequest};
use std::io::Write;

struct DebugRequest {
    handle: tokio_core::reactor::Handle,
}

impl Service for DebugRequest {
    type Request = GzWriterRequest<Body>;
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, (writer, req): Self::Request) -> Self::Future {
        match (req.method(), req.uri().path()) {
            (&Method::Get, "/")  => {
                let work = future::lazy(move || {
                        let mut writer = writer;
                        write!(&mut writer, "{:?}", req)?;
                        Ok(writer)
                    })
                    // .and_then(move || tokio_io::io::flush(writer, buf))
                    .and_then(|w| tokio_io::io::shutdown(w) )
                    .map(|_| ())
                    .map_err(|e| {
                        error!("error writing gzipped body: {:?}", e);
                    });
                self.handle.spawn(work);
                future::ok(Response::new())
            },
            _ =>
                future::ok(Response::new().with_status(StatusCode::NotFound)
            ),
        }
    }

}


fn main() {
    pretty_env_logger::init();
    let addr = "127.0.0.1:1337".parse().unwrap();
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let index_handle = core.handle();
    let serve_handle = core.handle();
    let serve = Http::new().serve_addr_handle(&addr, &serve_handle, move || {
        let svc = DebugRequest { handle: index_handle.clone() };
        let svc = GzWriterService::new(svc);
        Ok(svc)
    }).unwrap();
    println!("Listening on http://{} with 1 thread.", serve.incoming_ref().local_addr());
    let h2 = serve_handle.clone();
    serve_handle.spawn(serve.for_each(move |conn| {
        h2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));
    core.run(futures::future::empty::<(), ()>()).unwrap();
}
