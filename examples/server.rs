// #![deny(warnings)]
extern crate bytes;
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
use futures::future::{FutureResult, lazy};
use hyper::{Body, Method, Request, Response, StatusCode};
use hyper::server::{Http, Service};
use hyper_compress::stream;
use hyper_compress::server::{ChunkedGzWriterService, GzWriterRequest};

static INDEX: &'static [u8] = include_bytes!("test_file.txt");

struct Echo {
    handle: tokio_core::reactor::Handle,
}

impl Service for Echo {
    type Request = GzWriterRequest<Body>;
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, (writer, req): Self::Request) -> Self::Future {
        futures::future::ok(match (req.method(), req.uri().path()) {
            (&Method::Get, "/")  => {
                let work = tokio_io::io::write_all(writer, INDEX)
                    .and_then(|(w, _)| {
                        info!("shutting down io");
                        tokio_io::io::shutdown(w)
                    })
                    .map(|_| ())
                    .map_err(|e| {
                        error!("error writing gzipped body: {:?}", e);
                    });
                self.handle.spawn(work);
                Response::new()
            },
            _ => {
                Response::new().with_status(StatusCode::NotFound)
            }
        })
    }

}


fn main() {
    pretty_env_logger::init();
    let addr = "127.0.0.1:1337".parse().unwrap();
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let echo_handle = core.handle();
    let serve_handle = core.handle();
    let serve = Http::new().serve_addr_handle(&addr, &serve_handle, move || {
        let svc: ChunkedGzWriterService<Echo> =
            ChunkedGzWriterService::builder()
                .compression_level(flate2::Compression::fast())
                // .chunk_size()
                .to_service(Echo { handle: echo_handle.clone() });
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
