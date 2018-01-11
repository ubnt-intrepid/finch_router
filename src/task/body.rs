use std::marker::PhantomData;
use std::mem;
use futures::{Async, Future, Poll, Stream};
use futures::future::{self, FutureResult};
use http::{self, FromBody, HttpError};
use http::header::ContentLength;
use task::{Task, TaskContext};

#[allow(missing_docs)]
#[derive(Debug)]
pub struct Body<T, E> {
    _marker: PhantomData<fn() -> (T, E)>,
}

impl<T, E> Default for Body<T, E> {
    fn default() -> Self {
        Body {
            _marker: PhantomData,
        }
    }
}

impl<T, E> Task for Body<T, E>
where
    T: FromBody,
    E: From<T::Error>,
{
    type Item = T;
    type Error = E;
    type Future = BodyFuture<T, E>;

    fn launch(self, ctx: &mut TaskContext) -> Self::Future {
        if let Err(e) = T::validate(ctx.request()) {
            return BodyFuture::BadRequest(e.into());
        }

        let body = ctx.take_body().expect("cannot take the request body twice");
        let len = ctx.request()
            .header::<ContentLength>()
            .map_or(0, |&ContentLength(len)| len as usize);
        BodyFuture::Receiving(body, Vec::with_capacity(len))
    }
}

#[derive(Debug)]
pub enum BodyFuture<T, E> {
    BadRequest(E),
    Receiving(http::Body, Vec<u8>),
    Done(PhantomData<fn() -> (T, E)>),
}

impl<T, E> Future for BodyFuture<T, E>
where
    T: FromBody,
    E: From<T::Error>,
{
    type Item = T;
    type Error = Result<E, HttpError>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match mem::replace(self, BodyFuture::Done(PhantomData)) {
            BodyFuture::BadRequest(err) => Err(Ok(err)),
            BodyFuture::Receiving(mut body, mut buf) => loop {
                match body.poll().map_err(Err)? {
                    Async::Ready(Some(item)) => {
                        buf.extend_from_slice(&item);
                        continue;
                    }
                    Async::Ready(None) => {
                        let body = T::from_body(buf).map_err(Into::into).map_err(Ok)?;
                        break Ok(body.into());
                    }
                    Async::NotReady => {
                        *self = BodyFuture::Receiving(body, buf);
                        break Ok(Async::NotReady);
                    }
                }
            },
            BodyFuture::Done(..) => panic!("cannot resolve twice"),
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct BodyStream<E> {
    _marker: PhantomData<fn() -> E>,
}

impl<E> Default for BodyStream<E> {
    fn default() -> BodyStream<E> {
        BodyStream {
            _marker: PhantomData,
        }
    }
}

impl<E> Task for BodyStream<E> {
    type Item = http::Body;
    type Error = E;
    type Future = FutureResult<Self::Item, Result<Self::Error, HttpError>>;

    fn launch(self, ctx: &mut TaskContext) -> Self::Future {
        let body = ctx.take_body().expect("cannot take a body twice");
        future::ok(body)
    }
}
