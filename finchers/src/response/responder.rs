use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::string::ToString;
use std::sync::Arc;
use http::{Response, StatusCode};
use http::header;

use body::BodyStream;
use endpoint::Outcome;
use response::HttpStatus;

/// A trait to represents the conversion from outcome to an HTTP response.
pub trait Responder {
    type Item;

    /// Convert an outcome into an HTTP response
    fn respond(&self, outcome: Outcome<Self::Item>) -> Response<BodyStream>;
}

impl<R: Responder> Responder for Rc<R> {
    type Item = R::Item;

    fn respond(&self, outcome: Outcome<Self::Item>) -> Response<BodyStream> {
        (**self).respond(outcome)
    }
}

impl<R: Responder> Responder for Arc<R> {
    type Item = R::Item;

    fn respond(&self, outcome: Outcome<Self::Item>) -> Response<BodyStream> {
        (**self).respond(outcome)
    }
}

/// A pre-defined responder for creating an HTTP response by using `ToString::to_string`.
pub struct DefaultResponder<T> {
    _marker: PhantomData<fn(T)>,
}

impl<T> Copy for DefaultResponder<T> {}

impl<T> Clone for DefaultResponder<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Default for DefaultResponder<T> {
    fn default() -> Self {
        DefaultResponder {
            _marker: PhantomData,
        }
    }
}

impl<T> fmt::Debug for DefaultResponder<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DefaultResponder").finish()
    }
}

impl<T> Responder for DefaultResponder<T>
where
    T: HttpStatus + ToString,
{
    type Item = T;

    fn respond(&self, output: Outcome<T>) -> Response<BodyStream> {
        match output {
            Outcome::Ok(item) => respond_item(&item),
            Outcome::NoRoute => respond_noroute(),
            Outcome::Err(err) => respond_item(&*err),
        }
    }
}

fn respond_item<T>(item: &T) -> Response<BodyStream>
where
    T: ?Sized + ToString + HttpStatus,
{
    let body = item.to_string();
    Response::builder()
        .status(item.status_code())
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CONTENT_LENGTH, body.len().to_string().as_str())
        .body(body.into())
        .unwrap()
}

fn respond_noroute() -> Response<BodyStream> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Default::default())
        .unwrap()
}