//! Alpha Drive — ciphertext authority + client helpers.
//! Root capability stays on devices; the server never holds it.

pub mod client;
pub mod crypto;
pub mod entities;
pub mod media;
pub mod server;
pub mod store;
