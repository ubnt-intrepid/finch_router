//! The implementation of HTTP server based on hyper and tower-service.

mod builder;
mod error;
mod http_server;
pub mod middleware;

pub use self::builder::ServiceBuilder;
pub use self::error::{ServerError, ServerResult};
pub use self::http_server::ServerConfig;

// ==== start ====

use crate::app::App;
use crate::endpoint::Endpoint;
use crate::output::Output;

/// Create an instance of `ServiceBuilder` from the specified endpoint.
pub fn start<E>(endpoint: E) -> ServiceBuilder<App<E>>
where
    E: Endpoint,
    E::Output: Output,
{
    ServiceBuilder::new(App::new(endpoint))
}
