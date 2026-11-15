//! Link between thought (the deliberate decision layer, run in the harness) and the reflex
//! executor: a line-delimited JSON protocol over a Unix socket, served from the tick loop
//! through a thread-safe `Bridge`, plus an MCP tool server that speaks it.

pub mod bridge;
pub mod client;
pub mod decisions;
pub mod fake;
pub mod host;
pub mod mcp;
pub mod paths;
pub mod server;
pub mod wire;

pub use bridge::{Bridge, Ticket};
pub use client::Client;
pub use host::Host;
pub use paths::{default_socket_path, SOCKET_ENV};
pub use server::Server;
pub use wire::Request;
