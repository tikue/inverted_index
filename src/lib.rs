#![feature(unboxed_closures, core, iter_arith, custom_attribute, slice_patterns)]
extern crate itertools;
extern crate rustc_serialize;
extern crate core;

pub mod util;
pub mod index;

pub use index::*;
