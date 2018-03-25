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

pub struct Gzip<T> {
    inner: T,
    chunk_size: usize,
}

const CHUNK_SIZE: usize = 2048;

// struct EncodingStream<B: Buf> {
//     in_buf: Reader<B>,
//     encoder: Option<GzEncoder<Writer<BytesMut>>>
// }

// impl<B: Buf> Stream for EncodingStream<B> {
//     type Item = Bytes;
//     type Error = hyper::Error;

//     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
//         let mut chunk = [0; CHUNK_SIZE];
//         let readsz = self.in_buf.read(&mut chunk[..])?;
//         print!("readsz={:?};", readsz);
//         let mut encoder = self.encoder.take().expect("polled after finished");
//         print!(" didnt return");
//         let writesz = if readsz != 0 {
//             encoder.write(&chunk[0..readsz])?
//         } else {
//             0
//         };
//         println!(" writesz={:?};", writesz);
//         if writesz == 0 {
//             self.encoder.take().unwrap().try_finish().expect("finish on write 0");
//             return Ok(Async::Ready(None));
//         };
//         let buf = encoder.get_mut().get_mut().take().freeze();
//         self.encoder = Some(encoder);
//         Ok(Async::Ready(Some(buf)))
//     }
// }

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

struct ChunkingStream<B: Read> {
    read: B,
    chunksz: usize,
    buf: Cursor<Vec<u8>>,
}

impl<B: Read + fmt::Debug> Stream for ChunkingStream<B> {
    type Item = hyper::Chunk;
    type Error = hyper::Error;

     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut this_chunksz= 0;
        while this_chunksz < self.chunksz {

            debug!("read: {:?}", self.read);
            let mut buf = [0u8; CHUNK_SIZE];
            let read = self.read
                .read(&mut buf[..])
                .map_err(hyper::Error::Io)?;
            self.buf.write(&buf)
                .map_err(hyper::Error::Io)?;
            debug!("buffered {:?} bytes", read);
            if read == 0 { break; }
            this_chunksz += read;
        }
        if this_chunksz > 0 {
            let buf = mem::replace(&mut self.buf, Cursor::new(Vec::new())).into_inner();
            debug!("wrote {:?} byte chunk; len={:?}", this_chunksz, buf.len());
            Ok(Async::Ready(Some(hyper::Chunk::from(buf))))
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
                    buf: Cursor::new(Vec::new()),
                };
                Body::wrap_stream(stream)
            } else {
                body.into()
            };
            http::Response::from_parts(parts, body)
        }))
    }


}

