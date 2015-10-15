/// Contains extension traits related to BTreeMaps.
pub mod btree_map_ext;
/// Contains the Coalesce trait, for performing coalescence on collections and its items.
pub mod coalesce;
/// Contains the Successor trait, which is the same thing as `std::iter::Step`, except it's
/// implemented for chars.
pub mod successor;

pub use self::btree_map_ext::{BTreeMapExt, Intersection};
pub use self::coalesce::{Coalesce, Merge, MergeCoalesceMap};
pub use self::successor::Successor;
