
use futures::Future;
use flate2::Compression;
use hyper::{self, Body, Request, Response};
use hyper::header::{AcceptEncoding, ContentEncoding, Encoding, QualityItem};
use hyper::server::Service;

use ::stream;

/// A Hyper [`Body`] implementing [`io::Write`] which may or may not be
/// GZIP-compressed.
///
/// [`Body`]: https://docs.rs/hyper/0.11.24/hyper/struct.Body.html
/// [`io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
pub type GzBody = stream::MaybeGzWriter<stream::WriteBody>;

pub type GzWriterRequest<A> = (GzBody, Request<A>);

#[derive(Clone, Debug)]
pub struct GzWriterService<T> {
    inner: T,
    compression: Compression,
}

// ===== impl GzBody =====

impl GzBody {

    /// Construct a new `GzBody` for a [`Request`].
    ///
    /// # Arguments
    /// - `request`: The [`Request`] to construct a response body for.
    ///   The request's `Accept-Encoding` headers will be used to determine
    ///   whether or not the returned body may be compressed.
    /// - `level`: The [compression level] to use if the client will accept a
    ///   compressed response. When in doubt, try [`Compression::default()`].
    ///
    /// # Returns
    /// A writer paired with an associated `Body`. The `Body` should be set as
    /// the response body for the request, while the writer is to be used to
    /// write to the body asynchronously.
    ///
    /// [`Request`]: https://docs.rs/hyper/0.11.24/hyper/struct.Request.html
    /// [compression level]: https://docs.rs/flate2/1.0.1/flate2/struct.Compression.html
    /// [`Compression::default()`]: https://docs.rs/flate2/1.0.1/flate2/struct.Compression.html#impl-Default
    pub fn for_request<B>(request: &Request<B>,
                          level: Compression)
                          -> (Self, Body)
    {
        let (writer, body) = stream::WriteBody::new();
        let writer = if is_gzip(request) {
            stream::MaybeGzWriter::new(writer, level)
        } else {
            stream::MaybeGzWriter::new(writer, Compression::none())
        };
        (writer, body)
    }
}

/// A middleware that wraps another [`Service`] and provides it with a
/// writer for request bodies that may be GZIP compressed.
///
/// The wrapped service must take a request type consisting of
impl<T> GzWriterService<T> {

    pub fn new(inner: T) -> Self {
        GzWriterService {
            inner,
            compression: Compression::default(),
        }
    }

    pub fn with_compression(mut self, level: Compression) -> Self {
        self.compression = level;
        self
    }

}

fn is_gzip<B>(req: &Request<B>) -> bool {
    if let Some(accept_encodings) = req
        .headers()
        .get::<AcceptEncoding>()
    {
        return accept_encodings
            .iter()
            .any(|&QualityItem { ref item, .. }| item == &Encoding::Gzip)
    }
    false
}

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
