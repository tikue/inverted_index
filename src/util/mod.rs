/// Contains extension traits related to BTreeMaps.
pub mod btree_map_ext;
/// Contains the Coalesce trait, for performing coalescence on collections and its items.
pub mod coalesce;
/// Contains the Successor trait, which is the same thing as `std::iter::Step`, except it's
/// implemented for chars.
pub mod successor;

/// Utility functions for encoding and decoding utf-8 to and from bytes.
pub mod char_utf8;

pub use self::btree_map_ext::{BTreeMapExt, Intersection};
pub use self::coalesce::{Coalesce, Merge, MergeCoalesceMap};
pub use self::successor::Successor;
