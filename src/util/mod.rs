/// Contains extension traits related to BTreeMaps.
pub mod btree_map_ext;
/// Contains the Coalesce trait, for performing coalescence on collections and its items.
pub mod coalesce;

pub use self::btree_map_ext::{BTreeMapExt, Intersection};
pub use self::coalesce::{Coalesce, InsertSorted, Merge};
