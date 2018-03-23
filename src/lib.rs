extern crate bytes;
extern crate http;
extern crate hyper;
extern crate flate2;
extern crate futures;
extern crate tokio_io;

use std::fmt;
use std::io::{self, Read, Write};

use bytes::{BufMut, BytesMut};

use flate2::write::GzEncoder;
use futures::Poll;
use tokio_io::{AsyncRead, AsyncWrite, codec};


pub mod server;

/// A stream that may or may not be compressed.
pub enum CompressedBody<T: AsyncRead + AsyncWrite> {
    Uncompressed(hyper::Body),
    Gzip(codec::FramedRead<GzEncoder<T>, ChunkedCodec>),
    // TODO: other encodings?
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ChunkedCodec;

impl codec::Decoder for ChunkedCodec {
    type Item = hyper::Chunk;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut)
              -> Result<Option<hyper::Chunk>, io::Error> {
        if buf.len() > 0 {
            let len = buf.len();
            let buf = buf.split_to(len).freeze();
            Ok(Some(hyper::Chunk::from(buf)))
        } else {
            Ok(None)
        }
    }
}

impl codec::Encoder for ChunkedCodec {
    type Item = hyper::Chunk;
    type Error = io::Error;

    fn encode(&mut self, data: hyper::Chunk, buf: &mut BytesMut)
             -> Result<(), io::Error>
    {
        buf.reserve(data.len());
        buf.put(data.as_ref());
        Ok(())
    }
}

// // ===== impl MaybeCompressed =====


// impl<T: Write> fmt::Debug for MaybeCompressed<T> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match *self {
//             MaybeCompressed::Uncompressed(..) => f.pad("Uncompressed(..)"),
//             MaybeCompressed::Gzip(..) => f.pad("Gzip(..)"),
//         }
//     }
// }

// impl<T> Read for MaybeCompressed<T>
// where
//     T: Read + Write,
// {
//     #[inline]
//     fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
//         match *self {
//             MaybeCompressed::Uncompressed(ref mut s) => s.read(buf),
//             MaybeCompressed::Gzip(ref mut s) => s.read(buf),
//         }
//     }
// }

// impl<T> Write for MaybeCompressed<T>
// where
//     T: Read + Write,
// {
//     #[inline]
//     fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
//         match *self {
//             MaybeCompressed::Uncompressed(ref mut s) => s.write(buf),
//             MaybeCompressed::Gzip(ref mut s) => s.write(buf),
//         }
//     }

//     #[inline]
//     fn flush(&mut self) -> io::Result<()> {
//         match *self {
//             MaybeCompressed::Uncompressed(ref mut s) => s.flush(),
//             MaybeCompressed::Gzip(ref mut s) => s.flush(),
//         }
//     }
// }

// impl<T> AsyncRead for MaybeCompressed<T>
// where
//     T: AsyncRead + AsyncWrite,
//     GzEncoder<T>: AsyncRead,
// {
//     unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
//         match *self {
//             MaybeCompressed::Uncompressed(ref s) =>
//                 s.prepare_uninitialized_buffer(buf),
//             MaybeCompressed::Gzip(ref s) =>
//                 s.prepare_uninitialized_buffer(buf),
//         }
//     }
// }

// impl<T> AsyncWrite  for MaybeCompressed<T>
// where
//     T: AsyncRead + AsyncWrite,
//     GzEncoder<T>: AsyncWrite,
// {
//     fn shutdown(&mut self) -> Poll<(), io::Error> {
//         match *self {
//             MaybeCompressed::Uncompressed(ref mut s) => s.shutdown(),
//             MaybeCompressed::Gzip(ref mut s) => s.shutdown()
//         }
//     }
// }
