use bytes::{Buf, BufMut, Bytes, BytesMut};
use bytes::buf::{IntoBuf, Writer, Reader};
use futures::{future, executor, Async, Poll, Future, Stream};
use flate2::Compression;
use flate2::write::GzEncoder;
use hyper::{self, Body};
// use hyper::header;
use hyper::server::Service;
use http::{self, header};
use tokio_io::{AsyncRead, AsyncWrite};

// use ::MaybeCompressed;

use std::io::{Read, Write, Cursor, BufWriter};
use std::{mem, thread,fmt};
use std::iter::FromIterator;

pub struct Gzip<T> {
    inner: T,
    chunk_size: usize,
}

const CHUNK_SIZE: usize = 2048;

struct ChunkingStream<B: AsyncRead> {
    read: B,
    chunksz: usize,
}

impl<B: AsyncRead> Stream for ChunkingStream<B> {
    type Item = Bytes;
    type Error = hyper::Error;

     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut buf = [0u8; CHUNK_SIZE];
        let read = try_ready!(self.read
            .poll_read(&mut buf[..])
            .map_err(hyper::Error::Io));
        debug!("read {:?} bytes", read);

        if read > 0 {
            debug!("wrote {:?} byte chunk; len={:?}", read, buf.len());
            // i hate this
            let chunk = Bytes::from(&buf[..read]);
            Ok(Async::Ready(Some(chunk)))
        } else {
            debug!("reader emptied");
            Ok(Async::Ready(None))
        }

     }
}

impl<T> Gzip<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, chunk_size: CHUNK_SIZE, }
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
    B: Into<Body> + AsRef<[u8]> + Send + 'static,
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
        let chunksz = self.chunk_size;
        Box::new(self.inner.call(req).map(move |rsp| {
            let (mut parts, body) = rsp.into_parts();
            let body: Body = if is_gzip {
                let mut encoder = GzEncoder::new(
                    Cursor::new(Vec::<u8>::new()),
                    Compression::default());
                parts.headers.insert(
                    header::CONTENT_ENCODING,
                    "gzip".parse().unwrap());
                let n = encoder.write(body.as_ref())
                .expect("write to encoder");
                info!("wrote {:?}",n);
                let mut read = encoder.finish().expect("finish");
                read.set_position(0);
                let stream = ChunkingStream {
                    read,
                    chunksz,
                    // buf: Cursor::new(Vec::new()),
                };
                Body::wrap_stream(stream)
            } else {
                body.into()
            };
            http::Response::from_parts(parts, body)
        }))
    }


}

