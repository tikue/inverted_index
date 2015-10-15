use std::cmp::Ordering;
use std::collections::BTreeMap;

use util::*;

/// Information about the position of a single term within a document
#[derive(Copy, Clone, Debug, Hash, Eq, Ord, PartialEq, PartialOrd, RustcDecodable, RustcEncodable)]
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
    fn merge(self, other: Position) -> Option<Position> {
        if self.position == other.position {
            self.offsets.merge(other.offsets).map(|offsets| Position::new(offsets, self.position))
        } else {
            None
        }
    }
}

/// A postings map (doc id => positions) for a single term.
/// Records which Documents contain the term, and at which locations in the documents.
pub type PostingsMap = BTreeMap<usize, Vec<Position>>;

/// A MergeCoalesceMap for postings.
pub type MergePostingsMap = MergeCoalesceMap<usize, Vec<Position>>;

/// An extension trait for slices of `PostingsMap`s
/// that enables computing their intersection.
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

/// An extension trait for positionally intersecting two types. A positional intersection is
/// broadly defined as an intersection in which each element returned is close to an element
/// not in its own set.
pub trait PositionalIntersect {
    /// The return type of the positional intersection. Typically this will be the same type
    /// as the inputs, but for some cases it needs to be different, e.g. when the inputs are slices
    /// then the output will be an owned vec.
    type Intersection;

    /// Intersect positionally, returning an Intersection
    /// whose terms are present at position X in self's postings list for document D
    /// and position X + delta (for some delta) in the input's postings list for document D.
    fn intersect_positionally(&self, &Self) -> Self::Intersection;
}

impl PositionalIntersect for [Position] {
    type Intersection = Vec<Position>;

    fn intersect_positionally(&self, other: &[Position]) -> Vec<Position> {
        let mut intersection = vec![];
        let mut this = self.iter().cloned();
        let mut other = other.iter().cloned();
        let mut lval = this.next();
        let mut rval = other.next();
        loop {
            if let (Some(l), Some(r)) = (lval, rval) {
                match l.position.cmp(&r.position) {
                    Ordering::Less => {
                        if l.position + 1 == r.position {
                            if !intersection.is_empty() {
                                if intersection[intersection.len() - 1] != l {
                                    intersection.push(l);
                                }
                            } else {
                                intersection.push(l);
                            }
                            intersection.push(r);
                            rval = other.next();
                        }
                        lval = this.next();
                    }
                    Ordering::Greater | Ordering::Equal => rval = other.next(),
                }
            } else {
                return intersection;
            }
        }
    }
}

impl PositionalIntersect for PostingsMap {
    type Intersection = PostingsMap;
    fn intersect_positionally(&self, other: &Self) -> PostingsMap {
        let maps = &[self, other];
        maps.intersection()
            .map(|doc_id| {
                (doc_id.clone(),
                 self[doc_id].intersect_positionally(&other[doc_id]))
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use std::iter;
    use super::super::{MergePostingsMap, Position, PostingsMap};

    #[test]
    fn test_merge() {
        let postings = [iter::once((1, vec![Position::new((0, 1), 0), Position::new((2, 3), 1)]))
                            .collect::<PostingsMap>(),
                        iter::once((1, vec![Position::new((4, 5), 2), Position::new((6, 7), 3)]))
                            .collect()];
        assert_eq!(postings.iter().flat_map(|map| map).collect::<MergePostingsMap>().0,
                   iter::once((1, vec![Position::new((0, 1), 0),
                                       Position::new((2, 3), 1),
                                       Position::new((4, 5), 2),
                                       Position::new((6, 7), 3)]))
                       .collect());
    }
}
