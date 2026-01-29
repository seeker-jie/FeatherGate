pub mod config;
pub mod error;
pub mod types;
pub mod server;
pub mod providers;
pub mod metrics;

pub use error::FeatherGateError;
pub type Result<T> = std::result::Result<T, FeatherGateError>;
