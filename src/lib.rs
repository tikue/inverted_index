#![feature(unboxed_closures, core, iter_arith, custom_attribute, slice_patterns)]
#![deny(missing_docs)]

//! This crate contains tools for creating and working with the InvertedIndex struct.
//! Its primary utility is in search.

extern crate itertools;
extern crate rustc_serialize;
extern crate core;

/// Contains utility methods used in the rest of the crate.
pub mod util;
/// Contains the core primitives for use with InvertedIndex.
pub mod index;

pub use index::*;
