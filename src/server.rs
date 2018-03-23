use futures::{future, executor, Async, Future, Stream};
use flate2::Compression;
use flate2::read::GzEncoder;
use hyper::{self, Body};
// use hyper::header;
use hyper::server::Service;
use http::{self, header};
use tokio_io::{AsyncRead, AsyncWrite};

// use ::MaybeCompressed;

use std::io::{Read, Write, Cursor};
use std::thread;

pub struct Gzip<T> {
    inner: T,
}

// fn map_response<B, F>(rsp: http::Response<B>, f: F)
//     -> http::Response<MaybeCompressed<B>>
// where
//     F: Fn(B) -> MaybeCompressed<B>,
//     B: Write,
// {
//     let status = rsp.status();
//     let headers = rsp.headers().clone(); // TODO: remove clone.
//     let body: B = *(rsp.body_ref().unwrap().clone());
//     let body = f(body);
//     http::Response::new()
//         .with_status(status)
//         .with_headers(headers)
//         .with_body(body)
// }

impl<T> Gzip<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, }
    }
}

fn is_gzip<A>(req: &http::Request<A>) -> bool {
    // let accept_encodings = req.headers()
    //     .get::<header::AcceptEncoding>();
    // if accept_encodings.is_none() {
    //     return false;
    // }
    true
}

impl<T, A, B> Service for Gzip<T>
where
    T: Service<
        Request = http::Request<A>,
        Response = http::Response<B>,
        Error = hyper::Error,
    >,
    T::Future: Send + 'static,
    B: Into<Body> + AsRef<[u8]>,
    // MaybeCompressed<B>: AsyncRead,
{
    type Request = http::Request<A>;
    type Response = http::Response<Body>;
    type Error = hyper::Error;
    type Future = Box<Future<
        Item = http::Response<Body>,
        Error = hyper::Error,
    > + Send + 'static>;

    fn call(&self, req: http::Request<A>) -> Self::Future {
        let is_gzip = is_gzip(&req);
        Box::new(self.inner.call(req).map(move |rsp| {
            let (mut parts, body) = rsp.into_parts();
            let body: Body = if is_gzip {
                let mut encoder = GzEncoder::new(
                    body.as_ref(),
                    Compression::default());
                parts.headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                // let (mut tx_body, rx_body) = Body::channel();
                // executor::spawn(future::lazy(move || {
                //     let mut buf = [0u8; 4096];
                //     loop {
                //         match encoder.read(&mut buf) {
                //             Ok(n) if n == 0 => { break; },
                //             Ok(n) => {
                //                 let chunk: hyper::Chunk = buf[0..n].to_vec().into();
                //                 // while let Ok(Async::NotReady) = tx_body.poll_ready() { };
                //                 tx_body.send_data(chunk)
                //                     .expect("send data");
                //             },
                //             Err(e) => {
                //                 eprintln!("error: {:?}", e);
                //                 break;
                //                 }
                //         }
                //     }
                //     future::ok::<(),()>(())
                // }));
                let mut buf = Vec::new();
                encoder.read_to_end(&mut buf);
                buf.into()
            } else {
                body.into()
            };
            http::Response::from_parts(parts, body)
        }))
    }


}

