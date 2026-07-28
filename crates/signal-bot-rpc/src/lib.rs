//! RPC client for signal-cli.

pub mod client;
pub mod error;
pub mod methods;
pub mod types;

pub use client::{SignalCliClient, Transport};
pub use error::RpcError;
pub use types::*;
