
use futures::Future;
use flate2::Compression;
use hyper::{self, Body, Request, Response};
use hyper::header::{AcceptEncoding, TransferEncoding, ContentEncoding, Encoding, QualityItem};
use hyper::server::Service;

use ::stream;

#[derive(Clone, Copy, Debug, Default)]
pub struct Builder {
    chunk_size: Option<usize>,
    compression: Option<Compression>,
}

#[derive(Debug, Clone)]
pub struct ChunkedGzWriterService<T> {
    inner: T,
    chunk_size: usize,
    compression: Compression,
}

const DEFAULT_CHUNK_SIZE: usize = 1024;

// impl Builder
impl Builder {
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = Some(size);
        self
    }

    pub fn compression_level(mut self, compression: Compression) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn to_service<T>(&self, inner: T) -> ChunkedGzWriterService<T> {
        ChunkedGzWriterService {
            inner,
            chunk_size: self.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE),
            compression: self.compression.unwrap_or_default(),
        }
    }
}

impl ChunkedGzWriterService<()> {

    pub fn builder() -> Builder {
        Builder::default()
    }

}

impl<T> ChunkedGzWriterService<T> {

    pub fn new(inner: T) -> Self {
        Builder::default().to_service(inner)
    }

}

fn is_gzip<A>(req: &Request<A>) -> bool {
    if let Some(accept_encodings) = req
        .headers()
        .get::<AcceptEncoding>()
    {
        // TODO: honor quality items & stuff.
        return accept_encodings
            .iter()
            .any(|&QualityItem { ref item, .. }| item == &Encoding::Gzip)
    }
    false
}

pub type GzWriterRequest<A> = (stream::MaybeGzWriter<stream::Chunks>, Request<A>);

impl<T, A> Service for ChunkedGzWriterService<T>
where
    T: Service<
        Request = GzWriterRequest<A>,
        Response = Response<Body>,
        Error = hyper::Error,
    >,
    T::Future: Send + 'static,
{
    type Request = Request<A>;
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Box<Future<
        Item = Response<Body>,
        Error = hyper::Error,
    > + Send + 'static>;

    fn call(&self, req: Request<A>) -> Self::Future {
        let is_gzip = is_gzip(&req);
        let compression = if is_gzip {
            self.compression
        } else {
            Compression::none()
        };
        let (writer, body) = stream::Chunks::new(self.chunk_size);
        let writer = stream::MaybeGzWriter::new(writer, compression);
        Box::new(self.inner.call((writer, req)).map(move |resp| {
            if is_gzip {
                resp.with_header(ContentEncoding(vec![Encoding::Gzip]))
            } else {
                resp
            }.with_body(body)
        }))
    }

}
