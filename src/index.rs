use std::char;
use std::collections::Bound::{Included, Excluded};
use std::collections::BTreeMap;
use std::hash::Hasher;
use std::iter;
use std::str::{CharIndices, SplitWhitespace};
use std::ops;

use itertools::{GroupBy, Itertools};

use Query::*;
use super::*;
use util::*;

/// A basic implementation of an `Index`, the inverted index is a data structure that maps
/// from words to postings.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, RustcEncodable, RustcDecodable)]
pub struct InvertedIndex {
    // Maps terms to their postings
    index: BTreeMap<String, PostingsMap>,
    // Maps doc ids to their docs
    docs: BTreeMap<usize, Document>,
}

impl InvertedIndex {
    /// Constructs a new, empty InvertedIndex
    pub fn new() -> InvertedIndex {
        InvertedIndex {
            index: BTreeMap::new(),
            docs: BTreeMap::new(),
        }
    }

    /// Inserts the document.
    /// Insertings a document involves tokenizing the document's content
    /// and inserting each token into the index, pointing to the document and its position in the
    /// document.
    pub fn index(&mut self, doc: Document) {
        let previous_version = self.docs.insert(doc.id, doc.clone());
        if let Some(previous_version) = previous_version {
            let previous_analyzed = analyze_doc(previous_version.content());
            for (ngram, _) in previous_analyzed {
                let is_empty = {
                    let docs_for_ngram = self.index.get_mut(&ngram).unwrap();
                    docs_for_ngram.remove(&doc.id);
                    docs_for_ngram.is_empty()
                };
                if is_empty {
                    self.index.remove(&ngram);
                }
            }
        }

        let analyzed = analyze_doc(doc.content());
        for (ngram, position) in analyzed {
            self.index
                .entry(ngram)
                .or_insert_with(BTreeMap::new)
                .entry(doc.id)
                .or_insert_with(Vec::new)
                .search_coalesce(0, position);
        }
    }

    /// Performs a search to the specification of the given query
    pub fn query(&self, query: &Query) -> Vec<SearchResult> {
        let postings = self.query_rec(query);
        self.compute_results(postings)
    }

    /// A helper method for performing a Match query
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.query(&Match(query))
    }

    fn postings(&self, query: &str) -> PostingsMap {
        analyze_query(query).unique().flat_map(|word| self.index.get(&word)).merge_postings()
    }

    fn phrase(&self, phrase: &str) -> PostingsMap {
        let terms: Vec<_> = analyze_query(phrase).collect();
        let postings: Vec<_> = terms.windows(2)
                                    .map(|adjacent_terms| {
                                        let term0 = &adjacent_terms[0];
                                        let term1 = &adjacent_terms[1];
                                        if let (Some(posting0), Some(posting1)) =
                                               (self.index.get(term0), self.index.get(term1)) {
                                            posting0.intersect_positionally(posting1)
                                        } else {
                                            PostingsMap::new()
                                        }
                                    })
                                    .collect();
        postings.intersect_postings()
    }

    fn prefix(&self, prefix: &str) -> PostingsMap {
        let min = Included(prefix);
        let mut max: String = prefix.into();
        max.push(char::MAX);
        let max = Excluded(&max);
        self.index.range(min, max).map(|(_k, v)| v).merge_postings()
    }

    fn query_rec(&self, query: &Query) -> PostingsMap {
        match *query {
            Match(query) => self.postings(query),
            And(queries) => {
                let postings: Vec<_> = queries.iter().map(|q| self.query_rec(q)).collect();
                postings.intersect_postings()
            }
            Or(queries) => queries.into_iter().map(|q| self.query_rec(q)).merge_postings(),
            Phrase(phrase) => self.phrase(phrase),
            Prefix(prefix) => self.prefix(prefix),
        }
    }

    fn compute_results(&self, postings: PostingsMap) -> Vec<SearchResult> {
        let mut results: Vec<_> = postings.into_iter()
                                          .map(|(doc_id, positions)| {
                                              SearchResult::new(&self.docs[&doc_id], positions)
                                          })
                                          .collect();
        results.sort_by(|result1, result2| result2.score.partial_cmp(&result1.score).unwrap());
        results
    }
}

type Tokens<'a> = iter::FlatMap<
                iter::Enumerate<
                    iter::Filter<
                        GroupBy<bool, CharIndices<'a>, fn(&(usize, char)) -> bool>,
                        fn(&(bool, Vec<(usize, char)>)) -> bool>>,
                iter::Map<ops::Range<usize>, Ngrams>,
                fn((usize, (bool, Vec<(usize, char)>))) -> iter::Map<ops::Range<usize>, Ngrams>>;

type QueryTokens<'a> = iter::Map<SplitWhitespace<'a>, fn(&str) -> String>;

fn analyze_query(query: &str) -> QueryTokens {
    query.split_whitespace().map(str::to_lowercase)
}

fn analyze_doc(doc: &str) -> Tokens {
    doc.char_indices()
       .group_by(is_whitespace as fn(&(usize, char)) -> bool)
       .filter(not_whitespace as fn(&(bool, Vec<(usize, char)>)) -> bool)
       .enumerate()
       .flat_map(ngrams)
}

fn ngrams((position, (_, chars)): (usize, (bool, Vec<(usize, char)>)))
          -> iter::Map<ops::Range<usize>, Ngrams> {
    (1..chars.len() + 1).map(Ngrams::new(position, chars))
}

fn not_whitespace(&(is_whitespace, _): &(bool, Vec<(usize, char)>)) -> bool {
    !is_whitespace
}

fn is_whitespace(&(_, c): &(usize, char)) -> bool {
    c.is_whitespace()
}

struct Ngrams {
    position: usize,
    chars: Vec<(usize, char)>,
}

impl Ngrams {
    fn new(position: usize, chars: Vec<(usize, char)>) -> Ngrams {
        Ngrams {
            position: position,
            chars: chars,
        }
    }
}

impl Fn<(usize,)> for Ngrams {
    extern "rust-call" fn call(&self, (to,): (usize,)) -> (String, Position) {
        let word = self.chars[..to].iter().flat_map(|&(_, c)| c.to_lowercase()).collect();
        let start = self.chars[0].0;
        let (last_idx, last_char) = self.chars[to - 1];
        let finish = last_idx + last_char.len_utf8();
        (word, Position::new((start, finish), self.position))
    }
}

impl FnMut<(usize,)> for Ngrams {
    extern "rust-call" fn call_mut(&mut self, to: (usize,)) -> (String, Position) {
        self.call(to)
    }
}

impl FnOnce<(usize,)> for Ngrams {
    type Output = (String, Position);
    extern "rust-call" fn call_once(self, to: (usize,)) -> (String, Position) {
        self.call(to)
    }
}

#[cfg(test)]
mod test {
    use Query::*;
    use Document;
    use InvertedIndex;
    use Position;
    use SearchResult;
    use std::collections::BTreeMap;

    #[test]
    fn test_ngrams() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(1, "learn to program in rust today");
        let doc2 = Document::new(2, "what did you today do");
        index.index(doc1.clone());
        index.index(doc2.clone());
        let search_results = index.search("to");
        let expected: BTreeMap<_, _> =
            [(doc1.id.clone(),
              vec![Position::new((6, 8), 1), Position::new((25, 27), 5)]),
             (doc2.id.clone(), vec![Position::new((13, 15), 3)])]
                .iter()
                .cloned()
                .collect();
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[&search_result.doc.id])
        }
        assert_eq!("learn <span class=highlight>to</span> program in rust <span \
                    class=highlight>to</span>day",
                   search_results.iter()
                                 .find(|search_result| search_result.doc.id == doc1.id)
                                 .unwrap()
                                 .highlight("<span class=highlight>", "</span>"));

    }

    #[test]
    fn test_highlight() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(2, "Won\u{2019}t this split the ecosystem? Will everyone use?");
        index.index(doc1.clone());
        let expected = "Won\u{2019}t this split the *e*cosystem? Will *e*veryone use?";
        let search_results = index.search("e");
        assert_eq!(1, search_results.len());
        assert_eq!(search_results[0].highlight("*", "*"), expected);
    }

    #[test]
    fn test_unicode() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "嗨, 您好");
        index.index(doc.clone());
        let to_search = "您";
        let search_results = index.search(to_search);
        let &SearchResult { ref doc, ref positions, .. } = search_results.iter().next().unwrap();
        let Position{offsets:(begin, end), ..} = positions[0];
        assert_eq!(&doc.content()[begin..end], to_search);
    }

    #[test]
    fn test_update_doc() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "abc åäö");
        index.index(doc);
        let doc = Document::new(0, "different");
        index.index(doc);
        let search_results = index.search("å");
        assert!(search_results.is_empty());
        assert_eq!(index.docs.len(), 1);
    }

    #[test]
    fn test_ranking() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "beat");
        index.index(doc.clone());
        let doc2 = Document::new(1, "beast");
        index.index(doc2);
        let search_results = index.search("be");
        assert_eq!(index.docs.len(), 2);
        // "beat" should be first, since it's a closer match
        assert_eq!(search_results[0].doc.id, doc.id);
    }

    #[test]
    fn test_duplicate_term() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "beat");
        index.index(doc.clone());
        let search_results = index.search("be be");
        assert_eq!(search_results.len(), 1);
    }

    #[test]
    fn test_duplicate_term2() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "beat");
        index.index(doc.clone());
        let search_results = index.search("be b");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].positions, vec![Position::new((0, 2), 0)]);
    }

    #[test]
    fn test_lowercase_search() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "BeAt");
        index.index(doc.clone());
        let search_results = index.search("bE");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].positions, vec![Position::new((0, 2), 0)]);
    }

    #[test]
    fn test_lowercase_index() {
        let mut index = InvertedIndex::new();
        let doc = Document::new(0, "BeAt");
        index.index(doc.clone());
        let search_results = index.search("be");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].positions, vec![Position::new((0, 2), 0)]);
    }

    #[test]
    fn test_and() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(1, "learn to program in rust today");
        let doc2 = Document::new(2, "what did you today do");
        let doc3 = Document::new(3, "what did you do yesterday");
        index.index(doc1.clone());
        index.index(doc2.clone());
        index.index(doc3.clone());
        let search_results = index.query(&And(&[Match("today"), Match("you")]));
        let expected: BTreeMap<_, _> = [(doc2,
                                         vec![Position::new((9, 12), 2),
                                              Position::new((13, 18), 3)])]
                                           .iter()
                                           .cloned()
                                           .collect();
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[search_result.doc])
        }
    }

    #[test]
    fn test_and_or() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(1, "learn to program in rust today");
        let doc2 = Document::new(2, "what did you today do");
        let doc3 = Document::new(3, "what did you do yesterday");
        index.index(doc1.clone());
        index.index(doc2.clone());
        index.index(doc3.clone());
        let search_results = index.query(&Or(&[Match("you"),
                                               And(&[Match("today"), Match("you")])]));
        let expected: BTreeMap<_, _> = [(doc2.id,
                                         vec![Position::new((9, 12), 2),
                                              Position::new((13, 18), 3)]),
                                        (doc3.id, vec![Position::new((9, 12), 2)])]
                                           .iter()
                                           .cloned()
                                           .collect();
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[&search_result.doc.id])
        }
    }

    #[test]
    fn test_phrase() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(1, "learn to program in rust today");
        index.index(doc1.clone());
        let search_results = index.query(&Phrase("learn to program"));
        let expected: BTreeMap<_, _> = [(doc1.id.clone(),
                                         vec![Position::new((0, 5), 0),
                                              Position::new((6, 8), 1),
                                              Position::new((9, 16), 2)])]
                                           .iter()
                                           .cloned()
                                           .collect();
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[&search_result.doc.id]);
        }
        let search_results = index.query(&Phrase("lear t pro"));
        let expected: BTreeMap<_, _> = [(doc1.id,
                                         vec![Position::new((0, 4), 0),
                                              Position::new((6, 7), 1),
                                              Position::new((9, 12), 2)])]
                                           .iter()
                                           .cloned()
                                           .collect();
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[&search_result.doc.id]);
        }
    }

    #[test]
    fn test_phrase2() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(1, "is is is");
        index.index(doc1.clone());
        let expected: BTreeMap<_, _> = [(doc1.id.clone(),
                                         vec![Position::new((0, 1), 0),
                                              Position::new((3, 4), 1),
                                              Position::new((6, 7), 2)])]
                                           .iter()
                                           .cloned()
                                           .collect();
        let search_results = index.query(&Phrase("i i"));
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[&search_result.doc.id]);
        }
    }

    #[test]
    fn test_prefix() {
        let mut index = InvertedIndex::new();
        let doc1 = Document::new(1, "is is is");
        index.index(doc1.clone());
        let expected: BTreeMap<_, _> = [(doc1.id.clone(),
                                         vec![Position::new((0, 2), 0),
                                              Position::new((3, 5), 1),
                                              Position::new((6, 8), 2)])]
                                           .iter()
                                           .cloned()
                                           .collect();
        let search_results = index.query(&Prefix("i"));
        assert_eq!(search_results.len(), expected.len());
        for search_result in &search_results {
            assert_eq!(&search_result.positions, &expected[&search_result.doc.id]);
        }
    }
}
