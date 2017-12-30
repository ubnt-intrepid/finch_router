use std::collections::HashSet;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::thread;
use std::sync::Arc;

use futures::Stream;
use futures::stream::FuturesUnordered;
use hyper::Chunk;
use hyper::server::Http;
use net2::TcpBuilder;
#[cfg(unix)]
use net2::unix::UnixTcpBuilderExt;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};

use endpoint::Endpoint;
use responder::IntoResponder;
use super::{EndpointServiceFactory, ServiceFactory};

/// The factory of HTTP service
#[derive(Debug)]
pub struct ServerBuilder {
    addrs: Vec<SocketAddr>,
    num_workers: usize,
    proto: Http<Chunk>,
    secret_key: Option<Vec<u8>>,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        ServerBuilder {
            addrs: vec![],
            num_workers: 1,
            proto: Http::new(),
            secret_key: None,
        }
    }
}

impl ServerBuilder {
    /// Set the listener address of the service
    pub fn bind<S: ToSocketAddrs>(mut self, addr: S) -> Self {
        self.addrs.extend(addr.to_socket_addrs().unwrap());
        self
    }

    /// Set the number of worker threads
    pub fn num_workers(mut self, n: usize) -> Self {
        self.num_workers = n;
        self
    }

    /// Set the "raw" HTTP protocol
    pub fn proto(mut self, proto: Http<Chunk>) -> Self {
        self.proto = proto;
        self
    }

    /// Set the secret key used by `CookieManager`.
    pub fn secret_key<K: Into<Vec<u8>>>(mut self, key: K) -> Self {
        self.secret_key = Some(key.into());
        self
    }

    /// Start an HTTP server with given endpoint
    pub fn serve<E>(mut self, endpoint: E)
    where
        E: Endpoint + Send + Sync + 'static,
        E::Item: IntoResponder,
        E::Error: IntoResponder,
    {
        // create the factory of Hyper's service
        let factory = match self.secret_key {
            Some(key) => EndpointServiceFactory::with_secret_key(endpoint, &key),
            None => EndpointServiceFactory::new(endpoint),
        };

        // collect listener addresses and remove duplicated addresses.
        if self.addrs.is_empty() {
            self.addrs.push("0.0.0.0:4000".parse().unwrap());
            self.addrs.push("[::0]:4000".parse().unwrap());
        }
        let set: HashSet<_> = self.addrs.into_iter().collect();
        let addrs: Vec<_> = set.into_iter().collect();

        println!("Starting the server with following listener addresses:");
        for addr in &addrs {
            println!("- {}", addr);
        }

        // Now creates the context of worker threads and spawns them.
        let worker = Worker::new(factory, self.proto, addrs);
        for _ in 0..(self.num_workers - 1) {
            let worker = worker.clone();
            thread::spawn(move || {
                let _ = worker.run();
            });
        }
        let _ = worker.run();
    }
}

/// The context of worker threads
#[derive(Debug)]
pub struct Worker<F> {
    factory: Arc<F>,
    proto: Arc<Http<Chunk>>,
    addrs: Vec<SocketAddr>,
    capacity: i32,
}

impl<F> Clone for Worker<F> {
    fn clone(&self) -> Self {
        Worker {
            factory: self.factory.clone(),
            proto: self.proto.clone(),
            addrs: self.addrs.clone(),
            capacity: self.capacity,
        }
    }
}

impl<F> Worker<F>
where
    F: ServiceFactory,
    F::Service: 'static,
{
    #[allow(missing_docs)]
    pub fn new(factory: F, proto: Http<Chunk>, addrs: Vec<SocketAddr>) -> Self {
        Worker {
            factory: Arc::new(factory),
            proto: Arc::new(proto),
            addrs,
            capacity: 1024,
        }
    }

    #[allow(missing_docs)]
    pub fn capacity(&mut self, capacity: i32) {
        self.capacity = capacity;
    }

    #[allow(missing_docs)]
    pub fn run(&self) -> io::Result<()> {
        let mut core = Core::new()?;
        let handle = core.handle();

        let mut servers = FuturesUnordered::new();
        for addr in &self.addrs {
            let server = self.build_listener(addr, &handle)?
                .incoming()
                .for_each(|(sock, addr)| -> io::Result<()> {
                    let service = self.factory.new_service(&handle)?;
                    self.proto.bind_connection(&handle, sock, addr, service);
                    Ok(())
                });
            servers.push(server);
        }
        let server = servers.fold((), |(), _| -> io::Result<()> { Ok(()) });

        core.run(server)
    }

    fn build_listener(&self, addr: &SocketAddr, handle: &Handle) -> io::Result<TcpListener> {
        let listener = match *addr {
            SocketAddr::V4(..) => TcpBuilder::new_v4()?,
            SocketAddr::V6(..) => TcpBuilder::new_v6()?,
        };

        listener.reuse_address(true)?;
        #[cfg(unix)]
        listener.reuse_port(true)?;

        listener.bind(addr)?;
        let l = listener.listen(self.capacity)?;

        TcpListener::from_listener(l, addr, handle)
    }
}
