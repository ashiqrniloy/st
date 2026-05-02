mod client_to_server;
mod server_to_client;

use serde::{Deserialize, Serialize};

pub use client_to_server::ClientToServer;
pub use server_to_client::ServerToClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

#[cfg(test)]
mod tests;
