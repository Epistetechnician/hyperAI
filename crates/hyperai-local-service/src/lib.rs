//! Local `HyperAI` service, authentication, lifecycle, and metadata storage.

mod auth;
pub mod lifecycle;
mod market_history;
mod route_policy;
pub mod server;
pub mod storage;

pub use auth::{AuthState, SessionCookie};
