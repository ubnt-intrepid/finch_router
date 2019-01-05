use {
    crate::{
        common::Func,
        endpoint::{
            ActionContext, //
            ApplyContext,
            Endpoint,
            EndpointAction,
            IsEndpoint,
            Preflight,
        },
    },
    futures::Poll,
};

#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
pub struct Map<E, F> {
    pub(super) endpoint: E,
    pub(super) f: F,
}

impl<E: IsEndpoint, F> IsEndpoint for Map<E, F> {}

impl<E, F, Bd> Endpoint<Bd> for Map<E, F>
where
    E: Endpoint<Bd>,
    F: Func<E::Output> + Clone,
{
    type Output = (F::Out,);
    type Error = E::Error;
    type Action = MapAction<E::Action, F>;

    fn action(&self) -> Self::Action {
        MapAction {
            action: self.endpoint.action(),
            f: self.f.clone(),
        }
    }
}

#[derive(Debug)]
pub struct MapAction<A, F> {
    action: A,
    f: F,
}

impl<A, F, Bd> EndpointAction<Bd> for MapAction<A, F>
where
    A: EndpointAction<Bd>,
    F: Func<A::Output>,
{
    type Output = (F::Out,);
    type Error = A::Error;

    fn preflight(
        &mut self,
        cx: &mut ApplyContext<'_>,
    ) -> Result<Preflight<Self::Output>, Self::Error> {
        self.action
            .preflight(cx)
            .map(|x| x.map(|args| (self.f.call(args),)))
    }

    fn poll_action(&mut self, cx: &mut ActionContext<'_, Bd>) -> Poll<Self::Output, Self::Error> {
        self.action
            .poll_action(cx)
            .map(|x| x.map(|args| (self.f.call(args),)))
    }
}