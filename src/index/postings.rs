use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry::*;

use util::*;

/// Information about the position of a single term
/// within a document
#[derive(Clone, Debug)]
pub struct PositionInfo {
    /// Pairs of byte indexes into the document at the beginning and end of the term
    doc_offsets: Vec<(usize, usize)>,
    /// The token positions of the term.
    /// For example, for the sentence "I have to go to the store",
    /// the term "to" has positions [2, 4].
    token_positions: Vec<usize>
}

impl PositionInfo {
    /// Returns a new, empty, PositionInfo
    pub fn new() -> PositionInfo {
        PositionInfo {
            doc_offsets: vec![],
            token_positions: vec![],
        }
    }

    /// Merges the offsets and positions from one PositionInfo into itself.
    /// FIXME(tjk): this doesn't seem very useful, because if the position infos were
    /// for different terms, that information is lost after merging.
    pub fn merge(&mut self, other: &PositionInfo) {
        let mut last_idx = 0;
        for &offset in &other.doc_offsets {
            last_idx = self.doc_offsets.search_coalesce(last_idx, offset);
        }

        let mut last_idx = 0;
        for &position in &other.token_positions {
            last_idx = self.token_positions.insert_sorted(last_idx, position);
        }
    }
}

/// Each document is assigned a unique string id.
pub type DocId = String;

/// A Postings map (doc id => highlights) for a single term.
/// Records which Documents contain the term, and at which locations in the documents.
pub type PostingsMap = BTreeMap<DocId, PositionInfo>;


/// An extension trait for iterables over PostingsMaps
/// that enables the computation of the union of Postingsmaps
pub trait PostingsMerge {
    /// Computes the map containing the union of the maps in self
    fn merge_postings(self) -> PostingsMap;
}

impl<'a, Iter> PostingsMerge for Iter 
    where Iter: IntoIterator,
          Iter::Item: Borrow<PostingsMap> {
    fn merge_postings(self) -> PostingsMap {
        let mut map = PostingsMap::new();
        for tree in self {
            for (doc_id, position_info) in tree.borrow() {
                match map.entry(doc_id.clone()) {
                    Vacant(entry) => {
                        entry.insert(position_info.clone());
                    }
                    Occupied(mut entry) => entry.get_mut().merge(position_info),
                }
            }
        }
        map
    }
}


/// An extension trait for slices of PostingsMaps,
/// that enables the computation of the intersection
/// of PostingsMaps
pub trait PostingsIntersect {
    /// Computes the map containing the intersection of the the maps in self
    fn intersect_postings(self) -> PostingsMap;
}

impl<'a> PostingsIntersect for &'a [PostingsMap] {
    fn intersect_postings(self) -> PostingsMap {
        match self {
            [] => PostingsMap::new(),
            [ref posting] => posting.clone(),
            [ref posting0, rest..] => {
                self.intersection()
                    .map(|doc_id| {
                        let mut position_info = posting0[doc_id].clone();
                        for posting in rest {
                            position_info.merge(&posting[doc_id]);
                        }
                        (doc_id.clone(), position_info)
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::iter;
    use super::PostingsMerge;

    #[test]
    fn test_merge() {
        let postings = [iter::once(("1".into(), vec![(0, 1), (2, 3)])).collect(),
                        iter::once(("1".into(), vec![(4, 5), (6, 7)])).collect()];
        assert_eq!(postings.iter().merge_postings(), 
                   iter::once(("1".into(), vec![(0, 1), (2, 3), (4, 5), (6, 7)])).collect());
    }
}
