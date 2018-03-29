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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MaybeCompressed<A, B> {
    Uncompressed(A),
    Compressed(B),
}

#[derive(Clone, Debug)]
pub struct Chunks {
    size: usize,
    tx: mpsc::Sender<Result<Chunk, hyper::Error>>,
    state: State,
    buf: Option<Vec<u8>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum State {
    Ready,
    Sending(usize),
    Closing
}

// ===== impl Chunks =====

impl Chunks {

    pub fn new(size: usize) -> (Self, Body) {
        let (tx, rx) = Body::pair();
        let chunks = Chunks {
            size,
            buf: Some(vec![0u8; size]),
            tx,
            state: State::Ready,
        };
        (chunks, rx)
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


}

impl Write for Chunks {

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match self.state {
                State::Closing => {
                    trace!("Chunks::write: resume closing");
                    return Self::poll_to_result(self.tx.close())
                        .map(|_| 0);
                },
                State::Sending(amount)  => {
                    trace!("Chunks::write: sending {:?} bytes;", amount);
                    let _ = Self::poll_to_result(self.tx.poll_complete())?;
                    trace!("Chunks::write: sent");
                    self.state = State::Ready;
                    if self.buf.is_none() {
                        // sending started by this `write` call.
                        self.buf = Some(vec![0u8; self.size]);
                        trace!("Chunks::write: finished this write;");
                        return Ok(amount);
                    } else {
                        trace!("Chunks::write: ready for write;");
                        continue;
                    }
                },
                State::Ready => {
                    // Before trying to write, ensure the sender is ready.
                    let _ = Self::poll_to_result(self.tx.poll_ready())?;

                    let mut chunk = self.buf.take()
                        .expect("buf must be present in ready state");

                    let amount = chunk.write(buf)?;
                    trace!("Chunks::write: wrote {:?} bytes", amount);

                    if amount == 0 {
                        trace!("Chunks::write: start closing");
                        self.state = State::Closing;
                        continue;
                    };

                    let chunk = Chunk::from(chunk);
                    match self.tx.start_send(Ok(chunk)) {
                        Ok(AsyncSink::NotReady(_)) => {
                            trace!("Chunks::write: start_send -> NotReady; reset");
                            self.buf = Some(vec![0u8; self.size]);
                            return Err(io::Error::from(io::ErrorKind::WouldBlock));
                        },
                        Err(e) => {
                            trace!("Chunks::write: start_send -> Err");
                            return Err(io::Error::new(io::ErrorKind::Other, e))
                        },
                        Ok(AsyncSink::Ready) => {
                            trace!("Chunks::write: start sending");
                            self.state = State::Sending(amount);
                        },
                    };
                },
            }
        }

    }

    fn flush(&mut self) -> io::Result<()> {
        match self.state {
            State::Sending(amount) => {
                debug_assert!(self.buf.is_some());
                trace!("Chunks::flush: resume sending {:?} bytes;", amount);
                let _ = Self::poll_to_result(self.tx.poll_complete())?;
                trace!("Chunks::flush: sent");
                self.state = State::Ready;
            },
            State::Closing => {
                trace!("Chunks::flush: resume closing");
                Self::poll_to_result(self.tx.close())?;
            }
            State::Ready => {
                // Do nothing.
            }
        }
        Ok(())
    }
}

impl AsyncWrite for Chunks {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        trace!("Chunks::shutdown; state={:?};", self.state);
        if self.state != State::Closing {

            trace!("Chunks::shutdown; flushing before shutdown;");
            try_ready!(self.poll_flush());
            self.state == State::Closing;
        }
        self.poll_flush()
    }
}

// ===== impl MaybeCompressed =====


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
