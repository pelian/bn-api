// Quiet diesel warnings https://github.com/diesel-rs/diesel/issues/1785
#![allow(proc_macro_derive_resolution_fallback)]
#![cfg_attr(not(debug_assertions), deny(unused_variables))]
#![cfg_attr(not(debug_assertions), deny(unused_imports))]
#![cfg_attr(not(debug_assertions), deny(dead_code))]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![cfg_attr(not(debug_assertions), deny(unused_extern_crates))]
#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![recursion_limit = "256"]
#[macro_use]
extern crate diesel;
extern crate diesel_migrations;

extern crate argon2rs;
extern crate backtrace;
extern crate bigneon_http;
extern crate chrono;
extern crate chrono_tz;
extern crate hex;
extern crate itertools;
//#[macro_use]
extern crate log;
#[macro_use]
extern crate logging;

// This crate is only used in tests at the moment
#[allow(unused_extern_crates)]
#[cfg_attr(test, macro_use)]
extern crate macros;

extern crate rand;
extern crate ring;
#[macro_use]
extern crate embed_dirs_derive;
extern crate time;
extern crate uuid;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_with;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate validator_derive;
extern crate tari_client;
extern crate validator;

pub mod models;
pub mod schema;
pub mod services;
pub mod utils;
pub mod validators;

//#[cfg(test)]
mod test;

//#[cfg(test)]
pub mod dev {
    pub use test::*;
}

pub mod prelude {
    pub use models::*;
    pub use services::*;
    pub use utils::errors::*;
    pub use utils::*;
}
