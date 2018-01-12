//! A lancher of the HTTP services

pub mod backend;

use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use futures::{Future, Stream};
use hyper::{self, Chunk};
use hyper::server::NewService;
use tokio_core::reactor::{Core, Handle};

use endpoint::Endpoint;
use process::Process;
use responder::IntoResponder;
use service::EndpointServiceFactory;

pub use self::backend::TcpBackend;

/// HTTP-level configuration
#[derive(Debug)]
pub struct Http(::hyper::server::Http<Chunk>);

impl Default for Http {
    fn default() -> Self {
        Http(::hyper::server::Http::new())
    }
}

impl Http {
    /// Enable or disable `Keep-alive` option
    pub fn keep_alive(&mut self, enabled: bool) -> &mut Self {
        self.0.keep_alive(enabled);
        self
    }

    /// Enable pipeline mode
    pub fn pipeline(&mut self, enabled: bool) -> &mut Self {
        self.0.pipeline(enabled);
        self
    }
}

/// TCP level configuration
#[derive(Debug)]
pub struct Tcp<B = backend::DefaultBackend> {
    addrs: Vec<SocketAddr>,
    backend: B,
}

impl Default for Tcp<backend::DefaultBackend> {
    fn default() -> Self {
        Tcp {
            addrs: vec![],
            backend: Default::default(),
        }
    }
}

impl<B> Tcp<B> {
    /// Create a new instance of `Tcp` with given backend
    pub fn new(backend: B) -> Self {
        Tcp {
            backend,
            addrs: vec![],
        }
    }

    /// Set the listener addresses.
    pub fn set_addrs<S>(&mut self, addrs: S) -> io::Result<()>
    where
        S: ToSocketAddrs,
    {
        self.addrs = addrs.to_socket_addrs()?.collect();
        Ok(())
    }

    /// Returns the mutable reference of the inner backend
    pub fn backend(&mut self) -> &mut B {
        &mut self.backend
    }
}

/// Worker level configuration
#[derive(Debug)]
pub struct Worker {
    /// The number of worker threads
    pub num_workers: usize,
}

impl Default for Worker {
    fn default() -> Self {
        Worker { num_workers: 1 }
    }
}

/// The launcher of HTTP application.
#[derive(Debug)]
pub struct Application<S, B>
where
    S: NewService<Request = hyper::Request, Response = hyper::Response, Error = hyper::Error>,
    B: TcpBackend,
{
    /// The instance of `NewService`
    new_service: S,

    /// HTTP-level configuration
    proto: Http,

    /// TCP-level configuration
    tcp: Tcp<B>,

    /// The worker's configuration
    worker: Worker,
}

impl<S, B> Application<S, B>
where
    S: NewService<Request = hyper::Request, Response = hyper::Response, Error = hyper::Error>,
    B: TcpBackend,
{
    /// Create a new launcher from given service and TCP backend.
    pub fn from_service(new_service: S, backend: B) -> Self {
        Application {
            new_service,
            proto: Http::default(),
            worker: Worker::default(),
            tcp: Tcp {
                addrs: vec![],
                backend,
            },
        }
    }

    /// Returns a mutable reference of the service.
    pub fn new_service(&mut self) -> &mut S {
        &mut self.new_service
    }

    /// Returns a mutable reference of the HTTP configuration
    pub fn http(&mut self) -> &mut Http {
        &mut self.proto
    }

    /// Returns a mutable reference of the TCP configuration
    pub fn tcp(&mut self) -> &mut Tcp<B> {
        &mut self.tcp
    }

    /// Returns a mutable reference of the worker configuration
    pub fn worker(&mut self) -> &mut Worker {
        &mut self.worker
    }
}

impl<E, P> Application<EndpointServiceFactory<E, P>, backend::DefaultBackend>
where
    E: Endpoint,
    P: Process<In = E::Item, InErr = E::Error>,
    P::Out: IntoResponder,
    P::OutErr: IntoResponder,
{
    #[allow(missing_docs)]
    pub fn new(endpoint: E, process: P) -> Self {
        Self::from_service(
            EndpointServiceFactory::new(endpoint, process),
            Default::default(),
        )
    }
}

impl<S, B> Application<S, B>
where
    S: NewService<Request = hyper::Request, Response = hyper::Response, Error = hyper::Error> + Send + Sync + 'static,
    B: TcpBackend + Send + Sync + 'static,
{
    /// Start the HTTP server with given configurations
    pub fn run(mut self) {
        if self.tcp.addrs.is_empty() {
            println!("[info] Use default listener addresses.");
            self.tcp.addrs.push("0.0.0.0:4000".parse().unwrap());
            self.tcp.addrs.push("[::0]:4000".parse().unwrap());
        } else {
            let set: ::std::collections::HashSet<_> = self.tcp.addrs.into_iter().collect();
            self.tcp.addrs = set.into_iter().collect();
        }

        let ctx = Arc::new(WorkerContext {
            new_service: Arc::new(self.new_service),
            http: self.proto,
            tcp: self.tcp,
        });

        let mut handles = vec![];
        for _ in 0..self.worker.num_workers {
            let ctx = ctx.clone();
            handles.push(::std::thread::spawn(
                move || -> Result<(), ::hyper::Error> {
                    let mut core = Core::new()?;
                    let _ = ctx.spawn(&core.handle());
                    core.run(::futures::future::empty())
                },
            ));
        }

        for handle in handles {
            let _ = handle.join();
        }
    }
}

struct WorkerContext<S, B>
where
    S: NewService<Request = hyper::Request, Response = hyper::Response, Error = hyper::Error> + 'static,
    B: TcpBackend,
{
    new_service: Arc<S>,
    http: Http,
    tcp: Tcp<B>,
}

impl<S, B> WorkerContext<S, B>
where
    S: NewService<Request = hyper::Request, Response = hyper::Response, Error = hyper::Error> + 'static,
    B: TcpBackend,
{
    fn spawn(&self, handle: &Handle) -> Result<(), ::hyper::Error> {
        for addr in &self.tcp.addrs {
            let incoming = self.tcp.backend.incoming(addr, &handle)?;
            let serve = self.http
                .0
                .serve_incoming(incoming, self.new_service.clone())
                .for_each(|conn| conn.map(|_| ()))
                .map_err(|_| ());
            handle.spawn(serve);
        }

        Ok(())
    }
}