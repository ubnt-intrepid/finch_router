use std::rc::Rc;
use std::sync::Arc;
use futures::{future, Future, IntoFuture};
use http::{Error, Request};
use super::*;

/// Abstruction of an endpoint.
pub trait Endpoint {
    /// The type *on success*.
    type Item;

    /// The type *on failure*
    type Error;

    /// The type of returned value from `apply`.
    type Result: EndpointResult<Item = Self::Item, Error = Self::Error>;

    /// Validates the incoming HTTP request,
    /// and returns the instance of `Task` if matched.
    fn apply(&self, ctx: &mut EndpointContext) -> Option<Self::Result>;

    #[allow(missing_docs)]
    fn apply_request<R: Into<Request>>(&self, request: R) -> Option<<Self::Result as EndpointResult>::Future> {
        let mut request = request.into();
        self.apply(&mut EndpointContext::new(&request))
            .map(|result| result.into_future(&mut request))
    }

    #[allow(missing_docs)]
    fn join<T, E>(self, e: E) -> Join<Self, E::Endpoint>
    where
        Self: Sized,
        E: IntoEndpoint<T, Self::Error>,
    {
        join::join(self, e)
    }

    #[allow(missing_docs)]
    fn with<T, E>(self, e: E) -> With<Self, E::Endpoint>
    where
        Self: Sized,
        E: IntoEndpoint<T, Self::Error>,
    {
        with::with(self, e)
    }

    #[allow(missing_docs)]
    fn skip<T, E>(self, e: E) -> Skip<Self, E::Endpoint>
    where
        Self: Sized,
        E: IntoEndpoint<T, Self::Error>,
    {
        skip::skip(self, e)
    }

    #[allow(missing_docs)]
    fn or<E>(self, e: E) -> Or<Self, E::Endpoint>
    where
        Self: Sized,
        E: IntoEndpoint<Self::Item, Self::Error>,
    {
        or::or(self, e)
    }

    #[allow(missing_docs)]
    fn map<F, U>(self, f: F) -> Map<Self, F, U>
    where
        Self: Sized,
        F: Fn(Self::Item) -> U,
    {
        map::map(self, f)
    }

    #[allow(missing_docs)]
    fn map_err<F, U>(self, f: F) -> MapErr<Self, F, U>
    where
        Self: Sized,
        F: Fn(Self::Error) -> U,
    {
        map_err::map_err(self, f)
    }

    #[allow(missing_docs)]
    fn and_then<F, R>(self, f: F) -> AndThen<Self, F, R>
    where
        Self: Sized,
        F: Fn(Self::Item) -> R,
        R: IntoFuture<Error = Self::Error>,
    {
        and_then::and_then(self, f)
    }
}

impl<'a, E: Endpoint> Endpoint for &'a E {
    type Item = E::Item;
    type Error = E::Error;
    type Result = E::Result;

    fn apply(&self, ctx: &mut EndpointContext) -> Option<Self::Result> {
        (*self).apply(ctx)
    }
}

impl<E: Endpoint> Endpoint for Box<E> {
    type Item = E::Item;
    type Error = E::Error;
    type Result = E::Result;

    fn apply(&self, ctx: &mut EndpointContext) -> Option<Self::Result> {
        (**self).apply(ctx)
    }
}

impl<E: Endpoint> Endpoint for Rc<E> {
    type Item = E::Item;
    type Error = E::Error;
    type Result = E::Result;

    fn apply(&self, ctx: &mut EndpointContext) -> Option<Self::Result> {
        (**self).apply(ctx)
    }
}

impl<E: Endpoint> Endpoint for Arc<E> {
    type Item = E::Item;
    type Error = E::Error;
    type Result = E::Result;

    fn apply(&self, ctx: &mut EndpointContext) -> Option<Self::Result> {
        (**self).apply(ctx)
    }
}

/// Abstruction of returned value from an `Endpoint`.
pub trait EndpointResult {
    /// The type *on success*.
    type Item;

    /// The type *on failure*.
    type Error;

    /// The type of value returned from `launch`.
    type Future: Future<Item = Self::Item, Error = Result<Self::Error, Error>>;

    /// Launches itself and construct a `Future`, and then return it.
    ///
    /// This method will be called *after* the routing is completed.
    fn into_future(self, request: &mut Request) -> Self::Future;
}

impl<F: IntoFuture> EndpointResult for F {
    type Item = F::Item;
    type Error = F::Error;
    type Future = future::MapErr<F::Future, fn(F::Error) -> Result<F::Error, Error>>;

    fn into_future(self, _: &mut Request) -> Self::Future {
        self.into_future().map_err(Ok)
    }
}

/// Abstruction of types to be convert to an `Endpoint`.
pub trait IntoEndpoint<T, E> {
    /// The type of value returned from `into_endpoint`.
    type Endpoint: Endpoint<Item = T, Error = E>;

    /// Convert itself into `Self::Endpoint`.
    fn into_endpoint(self) -> Self::Endpoint;
}

impl<E, A, B> IntoEndpoint<A, B> for E
where
    E: Endpoint<Item = A, Error = B>,
{
    type Endpoint = E;

    #[inline]
    fn into_endpoint(self) -> Self::Endpoint {
        self
    }
}

impl<E> IntoEndpoint<(), E> for () {
    type Endpoint = EndpointOk<(), E>;

    fn into_endpoint(self) -> Self::Endpoint {
        ok(())
    }
}

impl<T, A, B> IntoEndpoint<Vec<A>, B> for Vec<T>
where
    T: IntoEndpoint<A, B>,
{
    type Endpoint = JoinAll<T::Endpoint>;

    fn into_endpoint(self) -> Self::Endpoint {
        join_all(self)
    }
}
