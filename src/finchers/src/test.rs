//! Helper functions for testing

#![allow(missing_docs)]

use std::io;
use tokio_core::reactor::Core;
use http::Request;
use core::{BodyStream, Error};
use endpoint::Endpoint;

#[derive(Debug)]
pub struct TestRunner<E: Endpoint> {
    endpoint: E,
    core: Core,
}

impl<E: Endpoint> TestRunner<E> {
    pub fn new(endpoint: E) -> io::Result<Self> {
        Ok(TestRunner {
            endpoint,
            core: Core::new()?,
        })
    }

    /// Apply an incoming HTTP request to the endpoint and return the result.
    ///
    /// # Panics
    /// This method will panic if an unexpected HTTP error will be occurred.
    pub fn run<R, B>(&mut self, request: R) -> Option<Result<E::Item, Error>>
    where
        R: Into<Request<B>>,
        B: Into<BodyStream>,
    {
        self.endpoint
            .apply_request(request)
            .map(|fut| self.core.run(fut))
    }
}

pub trait EndpointTestExt: Endpoint + sealed::Sealed {
    fn run<R, B>(&self, request: R) -> Option<Result<Self::Item, Error>>
    where
        R: Into<Request<B>>,
        B: Into<BodyStream>;
}

impl<E: Endpoint> EndpointTestExt for E {
    fn run<R, B>(&self, request: R) -> Option<Result<Self::Item, Error>>
    where
        R: Into<Request<B>>,
        B: Into<BodyStream>,
    {
        let mut runner = TestRunner::new(self).unwrap();
        runner.run(request)
    }
}

mod sealed {
    use endpoint::Endpoint;

    pub trait Sealed {}

    impl<E: Endpoint> Sealed for E {}
}