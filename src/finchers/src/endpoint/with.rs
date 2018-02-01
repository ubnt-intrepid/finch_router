#![allow(missing_docs)]

use endpoint::{Endpoint, EndpointContext, Input, IntoEndpoint};

pub fn with<E1, E2>(e1: E1, e2: E2) -> With<E1::Endpoint, E2::Endpoint>
where
    E1: IntoEndpoint,
    E2: IntoEndpoint,
{
    With {
        e1: e1.into_endpoint(),
        e2: e2.into_endpoint(),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct With<E1, E2> {
    e1: E1,
    e2: E2,
}

impl<E1, E2> Endpoint for With<E1, E2>
where
    E1: Endpoint,
    E2: Endpoint,
{
    type Item = E2::Item;
    type Result = E2::Result;

    fn apply(&self, input: &Input, ctx: &mut EndpointContext) -> Option<Self::Result> {
        let _f1 = try_opt!(self.e1.apply(input, ctx));
        let f2 = try_opt!(self.e2.apply(input, ctx));
        Some(f2)
    }
}