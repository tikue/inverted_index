use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry::*;

use util::*;

/// A Postings map (doc id => highlights) for a single term.
/// Records which Documents contain the term, and at which locations in the documents.
pub type PostingsMap = BTreeMap<String, Vec<(usize, usize)>>;


pub trait PostingsMerge {
    fn merge_postings(self) -> PostingsMap;
}

impl<'a, Iter> PostingsMerge for Iter 
    where Iter: Iterator,
          Iter::Item: Borrow<PostingsMap> {
    fn merge_postings(self) -> PostingsMap {
        let mut map = PostingsMap::new();
        for tree in self {
            let tree = tree.borrow();
            for (doc_id, highlights) in tree.iter() {
                match map.entry(doc_id.clone()) {
                    Vacant(entry) => {
                        entry.insert(highlights.clone());
                    }
                    Occupied(mut entry) => {
                        let entry = entry.get_mut();
                        for highlight in highlights {
                            if let Err(idx) = entry.binary_search(highlight) {
                                entry.coalesce(idx, highlight.clone());
                            }
                        }
                    }
                }
            }
        }
        map
    }
}


pub trait PostingsIntersect {
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
                            for highlight in &posting[doc_id] {
                                if let Err(idx) = highlights.binary_search(highlight) {
                                    highlights.coalesce(idx, *highlight);
                                }
                            }
                        }
                        (doc_id.clone(), highlights)
                    })
                    .collect()
            }
        }
    }
}


