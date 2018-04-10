extern crate finchers_core;
extern crate futures;
extern crate http;

use finchers_core::endpoint::{Endpoint, Error};
use finchers_core::input::{BodyStream, Input};
use finchers_core::util::create_task;
use futures::Future;
use http::header::{HeaderName, HeaderValue};
use http::{HttpTryFrom, Method, Request, Uri};
use std::mem;

#[derive(Debug)]
pub struct Client<E: Endpoint> {
    endpoint: E,
}

macro_rules! impl_constructors {
    ($($METHOD:ident => $name:ident,)*) => {$(
        pub fn $name<'a, U>(&'a self, uri: U) -> ClientRequest<'a, E>
        where
            Uri: HttpTryFrom<U>,
        {
            self.request(Method::$METHOD, uri)
        }
    )*};
}

impl<E: Endpoint> Client<E> {
    pub fn new(endpoint: E) -> Client<E> {
        Client { endpoint }
    }

    pub fn request<'a, M, U>(&'a self, method: M, uri: U) -> ClientRequest<'a, E>
    where
        Method: HttpTryFrom<M>,
        Uri: HttpTryFrom<U>,
    {
        let mut client = ClientRequest {
            client: self,
            request: Request::new(Default::default()),
        };
        client.method(method);
        client.uri(uri);
        client
    }

    impl_constructors! {
        GET => get,
        POST => post,
        PUT => put,
        HEAD => head,
        DELETE => delete,
        PATCH => patch,
    }
}

#[derive(Debug)]
pub struct ClientRequest<'a, E: Endpoint + 'a> {
    client: &'a Client<E>,
    request: Request<BodyStream>,
}

impl<'a, E: Endpoint> ClientRequest<'a, E> {
    pub fn method<M>(&mut self, method: M) -> &mut ClientRequest<'a, E>
    where
        Method: HttpTryFrom<M>,
    {
        *self.request.method_mut() = Method::try_from(method).ok().unwrap();
        self
    }

    pub fn uri<U>(&mut self, uri: U) -> &mut ClientRequest<'a, E>
    where
        Uri: HttpTryFrom<U>,
    {
        *self.request.uri_mut() = Uri::try_from(uri).ok().unwrap();
        self
    }

    pub fn header<K, V>(&mut self, name: K, value: V) -> &mut ClientRequest<'a, E>
    where
        HeaderName: HttpTryFrom<K>,
        HeaderValue: HttpTryFrom<V>,
    {
        let name = HeaderName::try_from(name).ok().unwrap();
        let value = HeaderValue::try_from(value).ok().unwrap();
        self.request.headers_mut().insert(name, value);
        self
    }

    pub fn body<B>(&mut self, body: B) -> &mut ClientRequest<'a, E>
    where
        B: Into<BodyStream>,
    {
        *self.request.body_mut() = body.into();
        self
    }

    pub fn run(&mut self) -> Result<E::Item, Error> {
        let ClientRequest { client, request } = mem::replace(
            self,
            ClientRequest {
                client: self.client,
                request: http::Request::new(Default::default()),
            },
        );

        let input: Input = request.into();
        let f = create_task(&client.endpoint, input);
        // TODO: replace with futures::executor
        let (result, _input) = f.wait().expect("EndpointTask never fails");
        result
    }
}