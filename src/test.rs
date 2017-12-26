//! Helper functions for testing

use tokio_core::reactor::Core;

use endpoint::{Endpoint, EndpointContext};
use http::{Body, Header, Method, Request};
use task::{Task, TaskContext};


/// A test case for `run_test()`
#[derive(Debug)]
pub struct TestCase {
    request: Request,
    body: Option<Body>,
}

impl TestCase {
    /// Construct a `TestCase` from given HTTP method and URI
    pub fn new(method: Method, uri: &str) -> Self {
        let request = Request::new(method, uri).expect("invalid URI");
        Self {
            request,
            body: None,
        }
    }

    /// Equivalent to `TestCase::new(Method::Get, uri)`
    pub fn get(uri: &str) -> Self {
        Self::new(Method::Get, uri)
    }

    /// Equivalent to `TestCase::new(Method::Post, uri)`
    pub fn post(uri: &str) -> Self {
        Self::new(Method::Post, uri)
    }

    /// Equivalent to `TestCase::new(Method::Put, uri)`
    pub fn put(uri: &str) -> Self {
        Self::new(Method::Put, uri)
    }

    /// Equivalent to `TestCase::new(Method::Delete, uri)`
    pub fn delete(uri: &str) -> Self {
        Self::new(Method::Delete, uri)
    }

    /// Equivalent to `TestCase::new(Method::Patch, uri)`
    pub fn patch(uri: &str) -> Self {
        Self::new(Method::Patch, uri)
    }

    /// Set the HTTP header of this test case
    pub fn with_header<H: Header>(mut self, header: H) -> Self {
        self.request.headers.set(header);
        self
    }

    /// Set the request body of this test case
    pub fn with_body<B: Into<Body>>(mut self, body: B) -> Self {
        self.body = Some(body.into());
        self
    }
}


/// Invoke given endpoint and return its result
pub fn run_test<T, E>(endpoint: T, input: TestCase) -> Option<Result<E::Item, E::Error>>
where
    T: AsRef<E>,
    E: Endpoint,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let TestCase { request, body } = input;

    let mut ctx = EndpointContext::new(&request, &handle);
    let task = endpoint.as_ref().apply(&mut ctx)?;

    let mut ctx = TaskContext::new(&request, &handle, body.unwrap_or_default());
    let result = core.run(task.launch(&mut ctx));

    Some(result)
}
