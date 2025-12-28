//! MDM Storage Layer
//!
//! Diesel-based storage for MDM enrollments, commands, and push certificates.

mod models;
mod schema;
mod sqlite;
mod traits;

pub use models::*;
pub use sqlite::SqliteStorage;
pub use traits::*;

use diesel_migrations::{EmbeddedMigrations, embed_migrations};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
