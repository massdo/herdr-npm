//! herdr-npm — npm scripts column for Herdr.
//!
//! Domain stays free of I/O. Application use cases talk to `HerdrPort` and
//! `ProjectPort`. Adapters own the socket, environment and TUI.

pub mod adapters;
pub mod application;
pub mod domain;

pub use application::ports::{HerdrPort, ProjectPort};
pub use domain::error::AppError;
