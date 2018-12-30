use super::Wrapper;
use crate::endpoint::{ApplyContext, ApplyResult, Endpoint};

/// Creates a wrapper for creating an endpoint which runs the provided function
/// before calling `Endpoint::apply()`.
pub fn before_apply<F>(f: F) -> BeforeApply<F>
where
    F: Fn(&mut ApplyContext<'_>) -> ApplyResult<()>,
{
    BeforeApply { f }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct BeforeApply<F> {
    f: F,
}

impl<'a, E, F> Wrapper<'a, E> for BeforeApply<F>
where
    E: Endpoint<'a>,
    F: Fn(&mut ApplyContext<'_>) -> ApplyResult<()> + 'a,
{
    type Output = E::Output;
    type Endpoint = BeforeApplyEndpoint<E, F>;

    fn wrap(self, endpoint: E) -> Self::Endpoint {
        BeforeApplyEndpoint {
            endpoint,
            f: self.f,
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
pub struct BeforeApplyEndpoint<E, F> {
    pub(super) endpoint: E,
    pub(super) f: F,
}

impl<'a, E, F> Endpoint<'a> for BeforeApplyEndpoint<E, F>
where
    E: Endpoint<'a>,
    F: Fn(&mut ApplyContext<'_>) -> ApplyResult<()> + 'a,
{
    type Output = E::Output;
    type Future = E::Future;

    #[inline]
    fn apply(&'a self, cx: &mut ApplyContext<'_>) -> ApplyResult<Self::Future> {
        (self.f)(cx)?;
        self.endpoint.apply(cx)
    }
}
