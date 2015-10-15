#![feature(plugin, unboxed_closures, core, iter_arith, custom_attribute, slice_patterns, collections_bound, btree_range)]
#![plugin(clippy)]
#![deny(missing_docs)]

//! # inverted_index
//! This library provides an in-memory (subject to change) `InvertedIndex` that indexes documents
//! to make them searchable.  Below are a few details about its features. For more examples, see
//! the tests.
//!
//! ## Indexing
//! ```
//! use inverted_index::{Document, InvertedIndex};
//! let mut index = InvertedIndex::new();
//! index.index(Document::new(1, "learn to program in rust today"));
//! ```
//!
//! Indexing is the process of inserting a document into the `InvertedIndex` to make it searchable.
//! The general process is:
//!
//! 1. Tokenize the document's text, typically by splitting the text on word boundaries.
//! 2. Insert each token into the index with the original document as its payload.
//! 3. Optionally store additional metadata along with each document, such as positional
//!    information.
//!
//! ## Searching
//! ```
//! use inverted_index::{Document, InvertedIndex};
//! let mut index = InvertedIndex::new();
//! index.index(Document::new(1, "learn to program in rust today"));
//! let results = index.search("prog");
//! ```
//!
//! Searches returns a set of search results. Each search result consists of a matching document,
//! the positions within the document that matched the query, and the document's search score.
//!
//! Searches can be performed via the `query` method using the composable `Query` enum, which
//! currently has four variants:
//!
//! * `Match` - The simplest query. Takes a string argument and returns any documents that match
//!             the string. `index.search(str)` is shorthand for `index.query(Match(str))`.
//! * `Phrase` - An exact-match query. Takes a string argument and returns any documents that
//!              contain the exact string. n.b. the `InvertedIndex` may return false positives in
//!              some cases.
//! * `And` - Composes a number of queries into a single query that restricts the results to the
//!           documents that are returned for each of the sub-queries.
//! * `Or` - Composes a number of queries into a single query that returns all the documents that
//!          are returned for any of the sub-queries.
//!
//! ## Scoring
//! The returned search results are ordered based on document relevance to the search query, sorted
//! descending. Currently, relevance for each document is computed based on the length of matching
//! content divided by the square root of the document length. This helps to ensure that longer
//! documents don't receive too unfair of an advantage over shorter documents.
//!
//! ## Highlighting
//! Search results include the positions in the document that matched the query. There is a helper
//! method defined on the `SearchResult` struct to highlight the matching content. It accepts
//! `before` and `after` string arguments to wrap the matching sections of the document in
//! highlights.
//!
//! ```
//! use inverted_index::{Document, InvertedIndex};
//! let mut index = InvertedIndex::new();
//! index.index(Document::new(1, "learn to program in rust today"));
//! let results = index.search("prog");
//! for search_result in &results {
//!     println!("{:?}", search_result.highlight("<b>", "</b>"));
//! }
//! ```

extern crate itertools;
extern crate rustc_serialize;
extern crate core;

/// Contains utility methods used in the rest of the crate.
pub mod util;

mod document;
mod index;
mod postings;
mod query;
mod search_result;

pub use index::InvertedIndex;
pub use document::Document;
pub use search_result::SearchResult;
pub use postings::{MergePostingsMap, PostingsMap, PostingsIntersect, PositionalIntersect, Position};
pub use query::Query;
