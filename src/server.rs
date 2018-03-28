use bytes::{Buf, BufMut, Bytes, BytesMut};
use bytes::buf::{IntoBuf, Writer, Reader};
use futures::{future, executor, Async, Poll, Future, Stream};
use flate2::Compression;
use flate2::write::GzEncoder;
use hyper::{self, Body};
use hyper::server::Service;
use http::{self, header};
use tokio_io::{AsyncRead, AsyncWrite};

use std::io::{Read, Write, Cursor, BufWriter};
use std::{mem, thread,fmt};
use std::iter::FromIterator;


#[derive(Clone, Copy, Debug, Default)]
pub struct Builder {
    chunk_size: Option<usize>,
    compression: Option<Compression>,
}

#[derive(Debug, Clone)]
pub struct GzipChunked<T> {
    inner: T,
    chunk_size: usize,
    compression: Compression,
}

const MAX_CHUNK_SIZE: usize = 2048;

struct ChunkingStream<B: AsyncRead> {
    read: B,
    chunksz: usize,
}

// impl Builder
impl Builder {
    pub fn chunk_size(self, size: usize) -> Self {
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = Some(size);
        self
    }

    pub fn compression_level(self, compression: Compression) -> Self {
        self.chunk_size = Some(size);
    pub fn compression_level(mut self, compression: Compression) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn to_service<T>(&self, inner: T) -> GzipChunked<T> {
        GzipChunked {
            inner,
            chunk_size: self.chunk_size.unwrap_or(MAX_CHUNK_SIZE),
            compression: self.compression.unwrap_or(Compression::Default)
            compression: self.compression.unwrap_or_default(),
        }
    }
}

impl<B: AsyncRead> Stream for ChunkingStream<B> {
    type Item = Bytes;
    type Error = hyper::Error;

     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut buf = [0u8; MAX_CHUNK_SIZE];
        let read = try_ready!(
            self.read
                .poll_read(&mut buf[..])
                .map_err(hyper::Error::Io)
        );
        trace!("ChunkingStream: read {:?} bytes", read);

        if read == 0 {
            trace!("ChunkingStream: reader emptied");
            return Ok(Async::Ready(None));
        }

        let chunk = Bytes::from(&buf[..read]);
        trace!("ChunkingStream: wrote {:?} byte chunk", chunk.len());
        Ok(Async::Ready(Some(chunk)))


     }
}

impl<T> GzipChunked<T> {
    pub fn new(inner: T) -> Self {
        Builder::default().to_service(inner)
    }
}

fn is_gzip<A>(req: &http::Request<A>) -> bool {
    if let Some(accept_encodings) = req
        .headers()
        .get(header::ACCEPT_ENCODING)
    {
        // TODO: honor quality items & stuff.
        return accept_encodings
            .to_str()
            .expect("accept-encoding header shouldn't be a weird wad of binary")
            .contains("gzip");
    }
    false
}

impl<T, A, B> Service for GzipChunked<T>
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
        trace!("GzipChunked: is_gzip={:?};", is_gzip);
        let chunksz = self.chunk_size;
        let compression = self.compression;

        Box::new(self.inner.call(req).map(move |rsp| {
        Box::new(self.inner.call(req).and_then(move |rsp| {
            let (mut parts, body) = rsp.into_parts();

            let body: Body = if is_gzip {
                let mut encoder = GzEncoder::new(
                    Cursor::new(Vec::<u8>::new()),
                    compression,
                );

                let n = encoder.write(body.as_ref())
                    .map_err(hyper::Error::Io)?;
                let mut read = encoder.finish()
                    .map_err(hyper::Error::Io)?;;
                trace!("GzipChunked: encoded {:?} bytes", n);

                read.set_position(0);
                let stream = ChunkingStream {
                    read,
                    chunksz,
                };

                parts.headers.insert(
                    header::CONTENT_ENCODING,
                    "gzip".parse().unwrap()
                );

                Body::wrap_stream(stream)
            } else {
                body.into()
            };
            http::Response::from_parts(parts, body)
            Ok(http::Response::from_parts(parts, body))
        }))
    }
}


}
