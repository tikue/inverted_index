use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry::*;

use util::*;

/// A Postings map (doc id => highlights) for a single term.
/// Records which Documents contain the term, and at which locations in the documents.
pub type PostingsMap = BTreeMap<String, Vec<(usize, usize)>>;


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
            for (doc_id, highlights) in tree.borrow() {
                match map.entry(doc_id.clone()) {
                    Vacant(entry) => {
                        entry.insert(highlights.clone());
                    }
                    Occupied(mut entry) => {
                        let entry = entry.get_mut();
                        let mut last_search = 0;
                        for &highlight in highlights {
                            last_search = entry.search_coalesce(last_search, highlight);
                        }
                    }
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
                        let mut highlights = posting0[doc_id].clone();
                        for posting in rest {
                            let mut last_search = 0;
                            for &highlight in &posting[doc_id] {
                                last_search = highlights.search_coalesce(last_search, highlight);
                            }
                        }
                        (doc_id.clone(), highlights)
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
