//! Hacker News–style demo library (shared by binary + integration tests).

pub mod app;
pub mod db;
pub mod entity;
pub mod migrate;
pub mod modules;
pub mod seed;

pub use app::{build_app, build_app_with_db};
pub use migrate::HnMigrator;
