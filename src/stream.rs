use flate2::Compression;
use flate2::write::GzEncoder;

use futures::{Async, AsyncSink, Poll, Sink};
use futures::sync::mpsc;

use hyper::{self, Body, Chunk};
use tokio_io::{AsyncRead, AsyncWrite};

use std::io::{self, Read, Write};

pub type MaybeGzWriter<W> = MaybeCompressed<W, GzEncoder<W>>;

impl<W: Write> MaybeGzWriter<W> {

    pub fn new(w: W, compression: Compression) -> Self {
        if compression == Compression::none() {
            return MaybeCompressed::Uncompressed(w);
        }
        MaybeCompressed::Compressed(GzEncoder::new(w, compression))
    }

}

impl<W: AsyncWrite> MaybeGzWriter<W> {
    /// Finish encoding this stream, returning the underlying writer once the
    /// encoding is done.
    ///
    /// Note that this function may not be suitable to call in a situation where
    /// the underlying stream is an asynchronous I/O stream. To finish a stream
    /// the `try_finish` (or `shutdown`) method should be used instead. To
    /// re-acquire ownership of a stream it is safe to call this method after
    /// `try_finish` or `shutdown` has returned `Ok`.
    ///
    /// # Errors
    ///
    /// This function will perform I/O to complete this stream, and any I/O
    /// errors which occur will be returned from this function.
    pub fn finish(self) -> io::Result<W> {
        match self {
            MaybeCompressed::Uncompressed(mut s) => {
                s.flush()?;
                Ok(s)
            },
            MaybeCompressed::Compressed(mut s) => {
                while s.shutdown()? == Async::NotReady {
                    trace!("MaybeGzWriter: still trying to shutdown io");
                    // continue trying to shutdown
                };
                s.finish()
            }
        }

    }

}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MaybeCompressed<A, B> {
    Uncompressed(A),
    Compressed(B),
}

#[derive(Clone, Debug)]
pub struct WriteBody {
    tx: mpsc::Sender<Result<Chunk, hyper::Error>>,
}


// ==== impl WriteBody =====

impl WriteBody {
    pub fn new() -> (Self, Body) {
        let (tx, rx) = Body::pair();
        let writer = WriteBody {
            tx,
        };
        (writer, rx)
    }
}

impl Write for WriteBody {

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        poll_to_result(self.tx.poll_ready())?;
        match self.tx.start_send(Ok(Chunk::from(Vec::from(buf)))) {
            Ok(AsyncSink::NotReady(_)) => {
                trace!("WriteBody::write: start_send -> NotReady");
                Err(io::Error::from(io::ErrorKind::WouldBlock))?
            },
            Err(e) => {
                trace!("WriteBody::write: start_send -> Err");
                Err(io::Error::new(io::ErrorKind::Other, e))?
            },
            Ok(AsyncSink::Ready) => {
                trace!("Chunks::write: start sending {:?} bytes", buf.len());
            },
        }
        poll_to_result(self.tx.poll_complete())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        poll_to_result(self.tx.poll_complete())
    }
}


impl AsyncWrite for WriteBody {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        trace!("WriteBody::shutdown;");
        self.poll_flush()
    }
}

#[inline]
fn poll_to_result<T>(poll: Poll<(), mpsc::SendError<T>>) -> io::Result<()>
where
    T: Send + Sync + 'static,
{
    match poll {
        Ok(Async::Ready(_)) => Ok(()),
        // If the sender is not ready, return WouldBlock to
        // signal that we're not ready.
        Ok(Async::NotReady) => Err(io::Error::from(io::ErrorKind::WouldBlock)),
        // SendError is returned if the body went away unexpectedly.
        // This is bad news.
        Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
    }
}


// ===== impl MaybeCompressed =====

impl<A, B> MaybeCompressed<A, B> {
    pub fn is_compressed(&self) -> bool {
        if let &MaybeCompressed::Compressed(_) = self {
            true
        } else {
            false
        }
    }
}

impl<A, B> Read for MaybeCompressed<A, B>
where
    A: Read,
    B: Read,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            MaybeCompressed::Uncompressed(ref mut s) => s.read(buf),
            MaybeCompressed::Compressed(ref mut s) => s.read(buf),
        }
    }
}

impl<A, B> Write for MaybeCompressed<A, B>
where
    A: Write,
    B: Write,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            MaybeCompressed::Uncompressed(ref mut s) => s.write(buf),
            MaybeCompressed::Compressed(ref mut s) => s.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match *self {
            MaybeCompressed::Uncompressed(ref mut s) => s.flush(),
            MaybeCompressed::Compressed(ref mut s) => s.flush(),
        }
    }
}

impl<A, B> AsyncRead for MaybeCompressed<A, B>
where
    A: AsyncRead,
    B: AsyncRead,
{
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match *self {
            MaybeCompressed::Uncompressed(ref s) => s.prepare_uninitialized_buffer(buf),
            MaybeCompressed::Compressed(ref s) => s.prepare_uninitialized_buffer(buf),
        }
    }
}
impl<A, B> AsyncWrite for MaybeCompressed<A, B>
where
    A: AsyncWrite,
    B: AsyncWrite,
{
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        trace!("MaybeCompressed::shutdown");
        match *self {
            MaybeCompressed::Uncompressed(ref mut s) => s.shutdown(),
            MaybeCompressed::Compressed(ref mut s) => s.shutdown(),
        }
    }
}
