// Quiet diesel warnings https://github.com/diesel-rs/diesel/issues/1785
#![allow(proc_macro_derive_resolution_fallback)]
// Force these as errors so that they are not lost in all the diesel warnings
#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![deny(unused_variables)]
#![deny(unused_imports)]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_extern_crates)]
#![deny(dead_code)]
extern crate bigneon_db;
extern crate chrono;
extern crate diesel;
extern crate rand;
#[macro_use]
extern crate serde_json;
extern crate time;
extern crate uuid;
extern crate validator;
//#[macro_use]
//extern crate macros;

mod unit;
