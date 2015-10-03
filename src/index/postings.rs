use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry::*;

use util::*;

/// Information about the position of a single term within a document
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, RustcEncodable)]
pub struct Position {
    /// Pair of byte indexes into the document at the beginning (inclusive) and end (exclusive) of 
    /// the term.
    pub offsets: (usize, usize),
    /// The token position of the term, i.e., the number of tokens that occur before it in the doc.
    /// For example, for the sentence "I have to go to the store",
    /// the term "to" has positions [2, 4].
    pub position: usize,
}

impl Position {
    /// Creates a new Position struct with the given offsets and position.
    pub fn new(offsets: (usize, usize), position: usize) -> Position {
        Position {
            offsets: offsets,
            position: position,
        }
    }
}

impl Merge for Position {
    fn merge(self, Position{offsets:(begin2, end2), position: position2}: Position)  -> Option<Position> {
        let Position{offsets: (begin1, end1), position: position1} = self;
        assert!(begin2 >= begin1);
        if position1 == position2 && end1 >= begin2 {
            Some(Position {
                offsets: if end1 < end2 { (begin1, end2) } else { (begin1, end1) },
                position: position1
            })
        } else {
            None
        }
    }
}

/// Each document is assigned a unique string id.
pub type DocId = String;

/// A Postings map (doc id => positions) for a single term.
/// Records which Documents contain the term, and at which locations in the documents.
pub type PostingsMap = BTreeMap<DocId, Vec<Position>>;

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
            for (doc_id, positions) in tree.borrow() {
                match map.entry(doc_id.clone()) {
                    Vacant(entry) => { entry.insert(positions.clone()); }
                    Occupied(mut entry) => entry.get_mut().merge_coalesce(positions.iter().cloned()),
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
                        let mut positions = posting0[doc_id].clone();
                        for posting in rest {
                            positions.merge_coalesce(posting[doc_id].iter().cloned());
                        }
                        (doc_id.clone(), positions)
                    })
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::iter;
    use super::super::Position;
    use super::PostingsMerge;

    #[test]
    fn test_merge() {
        let postings = [iter::once(("1".into(), vec![Position::new((0, 1), 0), Position::new((2, 3), 1)])).collect(),
                        iter::once(("1".into(), vec![Position::new((4, 5), 2), Position::new((6, 7), 3)])).collect()];
        assert_eq!(postings.iter().merge_postings(), 
                   iter::once(("1".into(), vec![Position::new((0, 1), 0), Position::new((2, 3), 1), Position::new((4, 5), 2), Position::new((6, 7), 3)])).collect());
    }
}
