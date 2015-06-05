#![feature(collections, unboxed_closures, core)]
extern crate itertools;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::iter;
use std::ops;

use itertools::{GroupBy, Itertools};
 
/// A Document contains an id and content.
/// Hashing and equality are based only on the id field.
#[derive(Clone, Eq, Debug)]
pub struct Document {
    id: String,
    content: String,
}

impl Drop for Document {
    fn drop(&mut self) {
        println!("Dropped doc {}", self.id);
    }
}

impl Document {
    /// Construct a new Document from an id and content.
    /// Both two arguments can be anything that can be turned into a String.
    pub fn new<S, T>(id: S, content: T) -> Document
        where S: Into<String>,
              T: Into<String> 
    {
        Document { id: id.into(), content: content.into(), }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}
 
impl Hash for Document {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        state.write(self.id.as_bytes());
    }
}
 
impl PartialEq<Document> for Document {
    fn eq(&self, other: &Document) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct IndexedDocument {
    pub doc: Arc<Document>,
    pub highlighted: (usize, usize),
}
 
fn ngrams((_, chars): (bool, Vec<(usize, char)>)) -> iter::Map<ops::Range<usize>, Ngrams> {
    (1..chars.len() + 1).map(Ngrams::new(chars))
}

fn not_whitespace(&(is_whitespace, _): &(bool, Vec<(usize, char)>)) -> bool {
    !is_whitespace
}

fn is_whitespace(&(_, c): &(usize, char)) -> bool {
    c.is_whitespace()
}

struct Ngrams {
    chars: Vec<(usize, char)>
}

impl Ngrams {
    fn new(chars: Vec<(usize, char)>) -> Ngrams {
        Ngrams { chars: chars }
    }
}

impl Fn<(usize,)> for Ngrams {
    extern "rust-call" fn call(&self, (to,): (usize,)) -> (String, (usize, usize)) {
        let word = self.chars[..to].iter().flat_map(|&(_, c)| c.to_lowercase()).collect();
        let start = self.chars[0].0;
        let (last_idx, last_char) = self.chars[to - 1];
        let finish = last_idx + last_char.len_utf8();
        (word, (start, finish))
    }
}

impl FnMut<(usize,)> for Ngrams {
    extern "rust-call" fn call_mut(&mut self, to: (usize,)) -> (String, (usize, usize)) {
        self.call(to)
    }
}

impl FnOnce<(usize,)> for Ngrams {
    type Output = (String, (usize, usize));
    extern "rust-call" fn call_once(self, to: (usize,)) -> (String, (usize, usize)) {
        self.call(to)
    }
}

fn analyze_doc(doc: &str) 
-> iter::FlatMap<
    iter::Filter<
        GroupBy<bool, std::str::CharIndices, fn(&(usize, char)) -> bool>, 
        fn(&(bool, Vec<(usize, char)>)) -> bool>, 
    iter::Map<ops::Range<usize>, Ngrams>,
    fn((bool, Vec<(usize, char)>)) -> iter::Map<ops::Range<usize>, Ngrams>> 
{
    doc.char_indices()
        .group_by(is_whitespace as fn(&(usize, char)) -> bool)
        .filter(not_whitespace as fn(&(bool, Vec<(usize, char)>)) -> bool)
        .flat_map(ngrams)
}

/// A basic implementation of an `Index`, the inverted index is a data structure that maps
/// from words to sets of Documents.
pub struct InvertedIndex {
    index: BTreeMap<String, HashSet<IndexedDocument>>,
    docs: BTreeMap<String, Arc<Document>>,
}
 
impl InvertedIndex {
    pub fn new() -> InvertedIndex {
        InvertedIndex {
            index: BTreeMap::new(),
            docs: BTreeMap::new(),
        }
    }

    /// A basic implementation of index, splits the document's content into whitespace-separated
    /// words, and inserts each word-document pair into the map.
    pub fn index(&mut self, doc: Document) {
        let doc = Arc::new(doc);
        let analyzed  = analyze_doc(doc.content());
        let previous_version = self.docs.insert(doc.id.clone(), doc.clone());
        if let Some(previous_version) = previous_version {
            let previous_analyzed = analyze_doc(previous_version.content());
            for (ngram, highlights) in previous_analyzed {
                let is_empty = {
                    let docs_for_ngram = self.index.get_mut(&ngram).unwrap();
                    docs_for_ngram.remove(&IndexedDocument { doc: doc.clone(), highlighted: highlights });
                    docs_for_ngram.is_empty()
                };
                if is_empty {
                    self.index.remove(&ngram);
                }
            }
        }

        for (ngram, highlighted) in analyzed {
            self.index.entry(ngram).or_insert_with(|| HashSet::new())
                .insert(IndexedDocument { doc: doc.clone(), highlighted: highlighted });
        }
    }
 
    /// A basic search implementation that splits the query's content into whitespace-separated
    /// words, looks up the set of Documents for each word, and then concatenates the sets.
    pub fn search(&self, query: &str) -> HashSet<SearchResult> {
        let map = query.split_whitespace()
            .flat_map(|word| self.index.get(&word.to_lowercase()))
            .flat_map(|docs| docs)
            .cloned()
            .fold(HashMap::new(), |mut map, search_result| {
                map.entry(search_result.doc)
                    .or_insert(Vec::new())
                    .push(search_result.highlighted);
                map
            });
        map.into_iter()
            .map(|(doc, mut highlights)| {
                highlights.sort();
                SearchResult { doc: doc, highlights: highlights }
            })
        .collect()
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct SearchResult {
    doc: Arc<Document>,
    highlights: Vec<(usize, usize)>,
}

impl SearchResult {
    #[cfg(test)]
    fn new(doc: Arc<Document>, highlights: Vec<(usize, usize)>) -> SearchResult {
        SearchResult {
            doc: doc,
            highlights: highlights,
        }
    }

    pub fn doc(&self) -> &Arc<Document> {
        &self.doc
    }

    pub fn highlights(&self) -> &Vec<(usize, usize)> {
        &self.highlights
    }

    pub fn highlighted_content(&self) -> String {
        let &SearchResult {ref doc, ref highlights} = self;
        let content = doc.content();
        let mut begin_idx = 0;
        let mut parts = vec![];
        for &(begin, end) in highlights {
            parts.push(&content[begin_idx..begin]);
            parts.push("<b>");
            parts.push(&content[begin..end]);
            parts.push("</b>");
            begin_idx = end;
        }
        parts.push(&content[begin_idx..]);
        parts.into_iter().join("")
    }
}

#[test]
fn test_search1() {
    let mut index = InvertedIndex::new();    
    let doc1 = Document::new("1", "learn to program in rust today");
    index.index(doc1.clone());
    let doc2 = Document::new("2", "what did you today do");
    index.index(doc2.clone());
    let search_results = index.search("today");
    let expected = [
        SearchResult::new(Arc::new(doc1), vec![(25, 30)]),
        SearchResult::new(Arc::new(doc2), vec![(13, 18)])
    ];
    assert_eq!(search_results, expected.iter().cloned().collect());
    assert_eq!("learn to program in rust <b>today</b>", expected[0].highlighted_content());
}

#[test]
fn test_ngrams() {
    let mut index = InvertedIndex::new();    
    let doc1 = Document::new("1", "learn to program in rust today");
    let doc2 = Document::new("2", "what did you today do");
    index.index(doc1.clone());
    index.index(doc2.clone());
    let search_results = index.search("to");
    let expected = [
        SearchResult::new(Arc::new(doc1), vec![(6, 8), (25, 27)]),
        SearchResult::new(Arc::new(doc2), vec![(13, 15)]),
    ];
    assert_eq!(search_results, expected.iter().cloned().collect());
    assert_eq!("learn <b>to</b> program in rust <b>to</b>day", expected[0].highlighted_content());

}

#[test]
fn test_search2() {
    let mut index = InvertedIndex::new();    
    let doc1 = Document::new("2", "what to do today");
    let doc2 = Document::new("3", "hey today");
    index.index(doc1.clone());
    index.index(doc2.clone());
    let search_results = index.search("to");
    let expected = [
        SearchResult::new(Arc::new(doc1), vec![(5, 7), (11, 13)]),
        SearchResult::new(Arc::new(doc2), vec![(4, 6)]),
    ];
    assert_eq!(search_results, expected.iter().cloned().collect());
    assert_eq!(expected[0].highlighted_content(), "what <b>to</b> do <b>to</b>day");
}

#[test]
fn test_unicode() {
    let mut index = InvertedIndex::new();    
    let doc = Document::new("0", "abc åäö");
    index.index(doc.clone());
    let to_search = "å";
    let search_results = index.search(to_search);
    let &SearchResult { ref doc, ref highlights } = search_results.iter().next().unwrap();
    let (begin, end) = highlights[0];
    assert_eq!(&doc.content()[begin..end], to_search);
}
