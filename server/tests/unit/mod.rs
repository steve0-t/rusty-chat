//! The default suite needs neither TLS certificates nor a running database.
//! Live database contracts are enabled explicitly with `--features db-tests`.

mod commands;
mod errors;
mod models;
mod tokens;

#[cfg(feature = "db-tests")]
mod database;
