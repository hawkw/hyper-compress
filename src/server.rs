
use futures::Future;
use flate2::Compression;
use hyper::{self, Body, Request, Response};
use hyper::header::{AcceptEncoding, ContentEncoding, Encoding, QualityItem};
use hyper::server::Service;

use ::stream;

#[derive(Clone, Copy, Debug, Default)]
pub struct Builder {
    compression: Option<Compression>,
}

#[derive(Debug, Clone)]
pub struct GzWriterService<T> {
    inner: T,
    compression: Compression,
}

// impl Builder
impl Builder {

    pub fn compression_level(mut self, compression: Compression) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn to_service<T>(&self, inner: T) -> GzWriterService<T> {
        GzWriterService {
            inner,
            compression: self.compression.unwrap_or_default(),
        }
    }
}

impl GzWriterService<()> {

    pub fn builder() -> Builder {
        Builder::default()
    }

}

impl<T> GzWriterService<T> {

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

pub type GzWriterRequest<A> = (stream::MaybeGzWriter<stream::WriteBody>, Request<A>);

impl<T, A> Service for GzWriterService<T>
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
        let (writer, body) = stream::WriteBody::new();
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
