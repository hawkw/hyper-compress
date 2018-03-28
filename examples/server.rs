#![deny(warnings)]
extern crate bytes;
extern crate futures;
extern crate hyper;
extern crate hyper_compress;
extern crate pretty_env_logger;
extern crate tokio;

use bytes::Bytes;

use futures::Future;
use futures::future::{FutureResult, lazy};

use hyper::{Body, Method, Request, Response, StatusCode};
use hyper::server::{Http, Service};
use hyper_compress::server::GzipChunked;

static INDEX: &'static [u8] =
    // b"Hello gzipped world!";
    br#####"

Docs.rs

    futures-0.2.0-beta
    Source
    Platform

Trait Stream
Associated Types
Item
Error
Required Methods
poll_next
Implementations on Foreign Types
AssertUnwindSafe<S>
Box<S>
&'a mut S
Recover<A, E, F>
Implementors

futures::stream
Structs

    AndThen
    BufferUnordered
    Buffered
    CatchUnwind
    Chain
    Chunks
    Collect
    Concat
    Empty
    ErrInto
    Filter
    FilterMap
    Flatten
    Fold
    ForEach
    Forward
    Fuse
    FuturesOrdered
    FuturesUnordered
    Inspect
    InspectErr
    IterOk
    IterResult
    Map
    MapErr
    Once
    OrElse
    Peekable
    PollFn
    Repeat
    ReuniteError
    Select
    SelectAll
    Skip
    SkipWhile
    SplitSink
    SplitStream
    StreamFuture
    Take
    TakeWhile
    Then
    Unfold
    Zip

Traits

    Stream
    StreamExt

Functions

    empty
    futures_ordered
    futures_unordered
    iter_ok
    iter_result
    once
    poll_fn
    repeat
    unfold

Trait futures::stream::Stream  [src]

pub trait Stream {
    type Item;
    type Error;
    fn poll_next(
        &mut self,
        cx: &mut Context
    ) -> Result<Async<Option<Self::Item>>, Self::Error>;
}



A stream of values produced asynchronously.

If Future is an asynchronous version of Result, then Stream is an asynchronous version of Iterator. A stream represents a sequence of value-producing events that occur asynchronously to the caller.

The trait is modeled after Future, but allows poll_next to be called even after a value has been produced, yielding None once the stream has been fully exhausted.
Errors

Streams, like futures, also bake in errors through an associated Error type. An error on a stream does not terminate the stream. That is, after one error is received, another value may be received from the same stream (it's valid to keep polling). Thus a stream is somewhat like an Iterator<Item = Result<T, E>>, and is always terminated by returning None.
Associated Types
type Item


Values yielded by the stream.
type Error


Errors yielded by the stream.
Required Methods
fn poll_next(
    &mut self,
    cx: &mut Context
) -> Result<Async<Option<Self::Item>>, Self::Error>


Attempt to pull out the next value of this stream, registering the current task for wakeup if the value is not yet available, and returning None if the stream is exhausted.
Return value

There are several possible return values, each indicating a distinct stream state:

    Ok(Pending) means that this stream's next value is not ready yet. Implementations will ensure that the current task will be notified when the next value may be ready.

    Ok(Ready(Some(val))) means that the stream has successfully produced a value, val, and may produce further values on subsequent poll_next calls.

    Ok(Ready(None)) means that the stream has terminated, and poll_next should not be invoked again.

    Err(err) means that the stream encountered an error while trying to poll_next. Subsequent calls to poll_next are allowed, and may return further values or errors.

Panics

Once a stream is finished, i.e. Ready(None) has been returned, further calls to poll_next may result in a panic or other "bad behavior". If this is difficult to guard against then the fuse adapter can be used to ensure that poll_next always returns Ready(None) in subsequent calls.
Implementations on Foreign Types
impl<S> Stream for AssertUnwindSafe<S> where
    S: Stream, [src]
[+] Expand description
impl<S> Stream for Box<S> where
    S: Stream + ?Sized, [src]
[+] Expand description
impl<'a, S> Stream for &'a mut S where
    S: Stream + ?Sized, [src]
[+] Expand description
impl<A, E, F> Stream for Recover<A, E, F> where
    A: Stream,
    F: FnMut(<A as Stream>::Error) -> Option<<A as Stream>::Item>, [src]
[+] Expand description
Implementors

    [src]
    impl Stream for Never  type Item = Never;  type Error = Never;
    [src]
    impl<A, B> Stream for Either<A, B> where
        A: Stream,
        B: Stream<Item = <A as Stream>::Item, Error = <A as Stream>::Error>,   type Item = <A as Stream>::Item;  type Error = <A as Stream>::Error;
    [src]
    impl<T> Stream for UnboundedReceiver<T>  type Item = T;  type Error = Never;
    [src]
    impl<T> Stream for Receiver<T>  type Item = T;  type Error = Never;
    [src]
    impl<S, R, P> Stream for Filter<S, R, P> where
        P: FnMut(&<S as Stream>::Item) -> R,
        R: IntoFuture<Item = bool, Error = <S as Stream>::Error>,
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, F> Stream for SinkMapErr<S, F> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, F> Stream for InspectErr<S, F> where
        F: FnMut(&<S as Stream>::Error),
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<I, E> Stream for IterOk<I, E> where
        I: Iterator,   type Item = <I as Iterator>::Item;  type Error = E;
    [src]
    impl<S, U, F> Stream for AndThen<S, U, F> where
        F: FnMut(<S as Stream>::Item) -> U,
        S: Stream,
        U: IntoFuture<Error = <S as Stream>::Error>,   type Item = <U as IntoFuture>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for Skip<S> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, F, U> Stream for MapErr<S, F> where
        F: FnMut(<S as Stream>::Error) -> U,
        S: Stream,   type Item = <S as Stream>::Item;  type Error = U;
    [src]
    impl<F> Stream for IntoStream<F> where
        F: Future,   type Item = <F as Future>::Item;  type Error = <F as Future>::Error;
    [src]
    impl<T, E> Stream for Repeat<T, E> where
        T: Clone,   type Item = T;  type Error = E;
    [src]
    impl<I, T, E> Stream for IterResult<I> where
        I: Iterator<Item = Result<T, E>>,   type Item = T;  type Error = E;
    [src]
    impl<S> Stream for Buffered<S> where
        S: Stream,
        <S as Stream>::Item: IntoFuture,
        <<S as Stream>::Item as IntoFuture>::Error == <S as Stream>::Error,   type Item = <<S as Stream>::Item as IntoFuture>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for SplitStream<S> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for CatchUnwind<S> where
        S: Stream + UnwindSafe,   type Item = Result<<S as Stream>::Item, <S as Stream>::Error>;  type Error = Box<Any + 'static + Send>;
    [src]
    impl<S, U, Fut, F> Stream for With<S, U, Fut, F> where
        F: FnMut(U) -> Fut,
        Fut: IntoFuture,
        S: Stream + Sink,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, U, F> Stream for Then<S, U, F> where
        F: FnMut(Result<<S as Stream>::Item, <S as Stream>::Error>) -> U,
        S: Stream,
        U: IntoFuture,   type Item = <U as IntoFuture>::Item;  type Error = <U as IntoFuture>::Error;
    [src]
    impl<S> Stream for BufferUnordered<S> where
        S: Stream,
        <S as Stream>::Item: IntoFuture,
        <<S as Stream>::Item as IntoFuture>::Error == <S as Stream>::Error,   type Item = <<S as Stream>::Item as IntoFuture>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, U, F> Stream for OrElse<S, U, F> where
        F: FnMut(<S as Stream>::Error) -> U,
        S: Stream,
        U: IntoFuture<Item = <S as Stream>::Item>,   type Item = <S as Stream>::Item;  type Error = <U as IntoFuture>::Error;
    [src]
    impl<T, E, F> Stream for PollFn<F> where
        F: FnMut(&mut Context) -> Result<Async<Option<T>>, E>,   type Item = T;  type Error = E;
    [src]
    impl<S, U, St, F> Stream for WithFlatMap<S, U, St, F> where
        F: FnMut(U) -> St,
        S: Stream + Sink,
        St: Stream<Item = <S as Sink>::SinkItem, Error = <S as Sink>::SinkError>,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, R, P> Stream for SkipWhile<S, R, P> where
        P: FnMut(&<S as Stream>::Item) -> R,
        R: IntoFuture<Item = bool, Error = <S as Stream>::Error>,
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, E> Stream for SinkErrInto<S, E> where
        S: Stream + Sink,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S1, S2> Stream for Select<S1, S2> where
        S1: Stream,
        S2: Stream<Item = <S1 as Stream>::Item, Error = <S1 as Stream>::Error>,   type Item = <S1 as Stream>::Item;  type Error = <S1 as Stream>::Error;
    [src]
    impl<F> Stream for FlattenStream<F> where
        F: Future,
        <F as Future>::Item: Stream,
        <<F as Future>::Item as Stream>::Error == <F as Future>::Error,   type Item = <<F as Future>::Item as Stream>::Item;  type Error = <<F as Future>::Item as Stream>::Error;
    [src]
    impl<S, E> Stream for ErrInto<S, E> where
        S: Stream,
        <S as Stream>::Error: Into<E>,   type Item = <S as Stream>::Item;  type Error = E;
    [src]
    impl<T> Stream for FuturesUnordered<T> where
        T: Future,   type Item = <T as Future>::Item;  type Error = <T as Future>::Error;
    [src]
    impl<T, E> Stream for Empty<T, E>  type Item = T;  type Error = E;
    [src]
    impl<S> Stream for Take<S> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for Peekable<S> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, R, F, B> Stream for FilterMap<S, R, F> where
        F: FnMut(<S as Stream>::Item) -> R,
        R: IntoFuture<Item = Option<B>, Error = <S as Stream>::Error>,
        S: Stream,   type Item = B;  type Error = <S as Stream>::Error;
    [src]
    impl<F> Stream for Once<F> where
        F: Future,   type Item = <F as Future>::Item;  type Error = <F as Future>::Error;
    [src]
    impl<S1, S2> Stream for Chain<S1, S2> where
        S1: Stream,
        S2: Stream<Item = <S1 as Stream>::Item, Error = <S1 as Stream>::Error>,   type Item = <S1 as Stream>::Item;  type Error = <S1 as Stream>::Error;
    [src]
    impl<S, R, P> Stream for TakeWhile<S, R, P> where
        P: FnMut(&<S as Stream>::Item) -> R,
        R: IntoFuture<Item = bool, Error = <S as Stream>::Error>,
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<T> Stream for FuturesOrdered<T> where
        T: Future,   type Item = <T as Future>::Item;  type Error = <T as Future>::Error;
    [src]
    impl<S1, S2> Stream for Zip<S1, S2> where
        S1: Stream,
        S2: Stream<Error = <S1 as Stream>::Error>,   type Item = (<S1 as Stream>::Item, <S2 as Stream>::Item);  type Error = <S1 as Stream>::Error;
    [src]
    impl<S> Stream for Flatten<S> where
        S: Stream,
        <S as Stream>::Item: Stream,
        <<S as Stream>::Item as Stream>::Error: From<<S as Stream>::Error>,   type Item = <<S as Stream>::Item as Stream>::Item;  type Error = <<S as Stream>::Item as Stream>::Error;
    [src]
    impl<S, F, U> Stream for Map<S, F> where
        F: FnMut(<S as Stream>::Item) -> U,
        S: Stream,   type Item = U;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for SelectAll<S> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<T, F, Fut, It> Stream for Unfold<T, F, Fut> where
        F: FnMut(T) -> Fut,
        Fut: IntoFuture<Item = Option<(It, T)>>,   type Item = It;  type Error = <Fut as IntoFuture>::Error;
    [src]
    impl<S> Stream for Buffer<S> where
        S: Sink + Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for Fuse<S> where
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S, F> Stream for Inspect<S, F> where
        F: FnMut(&<S as Stream>::Item),
        S: Stream,   type Item = <S as Stream>::Item;  type Error = <S as Stream>::Error;
    [src]
    impl<S> Stream for Chunks<S> where
        S: Stream,   type Item = Vec<<S as Stream>::Item>;  type Error = <S as Stream>::Error;

"#####;

struct Echo;

impl Service for Echo {
    type Request = Request<Body>;
    type Response = Response<Bytes>;
    type Error = hyper::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        futures::future::ok(match (req.method(), req.uri().path()) {
            (&Method::GET, "/")  => {
                Response::new(INDEX.into())
            },
            _ => {
                let mut res = Response::new(Bytes::new());
                *res.status_mut() = StatusCode::NOT_FOUND;
                res
            }
        })
    }

}


fn main() {
    pretty_env_logger::init();
    let addr = "127.0.0.1:1337".parse().unwrap();

    tokio::run(lazy(move || {
        let server = Http::new().bind(&addr, || {
            let svc: GzipChunked<Echo> =
                GzipChunked::builder()
                    .chunk_size(128)
                    .to_service(Echo);
            Ok(svc)
        }).unwrap();
        println!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
        server.run().map_err(|err| eprintln!("Server error {}", err))
    }));
}
