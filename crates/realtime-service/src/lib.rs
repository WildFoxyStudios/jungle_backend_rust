//! Library surface for `realtime-service`.
//!
//! Exposes the connection hub, event consumer, handlers, and routes so
//! integration tests (and other crates, if needed) can exercise the realtime
//! plumbing without re-implementing it.
pub mod event_consumer;
pub mod handlers;
pub mod hub;
pub mod routes;
