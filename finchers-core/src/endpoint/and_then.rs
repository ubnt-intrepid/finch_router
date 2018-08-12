use std::future::Future;
use std::mem::PinMut;
use std::task;
use std::task::Poll;

use futures_core::future::TryFuture;
use pin_utils::unsafe_pinned;

use endpoint::Endpoint;
use error::Error;
use generic::{Func, Tuple};
use input::{Cursor, Input};

use super::try_chain::{TryChain, TryChainAction};

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
pub struct AndThen<E, F> {
    pub(super) endpoint: E,
    pub(super) f: F,
}

impl<E, F> Endpoint for AndThen<E, F>
where
    E: Endpoint,
    F: Func<E::Output> + Clone,
    F::Out: TryFuture<Error = Error>,
    <F::Out as TryFuture>::Ok: Tuple,
{
    type Output = <F::Out as TryFuture>::Ok;
    type Future = AndThenFuture<E::Future, F::Out, F>;

    fn apply(&self, input: PinMut<Input>, cursor: Cursor) -> Option<(Self::Future, Cursor)> {
        let (f1, cursor) = self.endpoint.apply(input, cursor)?;
        let f = self.f.clone();
        Some((
            AndThenFuture {
                try_chain: TryChain::new(f1, f),
            },
            cursor,
        ))
    }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct AndThenFuture<F1, F2, F>
where
    F1: TryFuture<Error = Error>,
    F2: TryFuture<Error = Error>,
    F: Func<F1::Ok, Out = F2>,
    F1::Ok: Tuple,
    <F::Out as TryFuture>::Ok: Tuple,
{
    try_chain: TryChain<F1, F2, F>,
}

impl<F1, F2, F> AndThenFuture<F1, F2, F>
where
    F1: TryFuture<Error = Error>,
    F2: TryFuture<Error = Error>,
    F: Func<F1::Ok, Out = F2>,
    F1::Ok: Tuple,
    <F::Out as TryFuture>::Ok: Tuple,
{
    unsafe_pinned!(try_chain: TryChain<F1, F2, F>);
}

impl<F1, F2, F> Future for AndThenFuture<F1, F2, F>
where
    F1: TryFuture<Error = Error>,
    F2: TryFuture<Error = Error>,
    F: Func<F1::Ok, Out = F2>,
    F1::Ok: Tuple,
    <F::Out as TryFuture>::Ok: Tuple,
{
    type Output = Result<F2::Ok, Error>;

    fn poll(mut self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        self.try_chain().poll(cx, |result, f| match result {
            Ok(ok) => TryChainAction::Future(f.call(ok)),
            Err(err) => TryChainAction::Output(Err(err)),
        })
    }
}
