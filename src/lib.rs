extern crate itertools;

use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use itertools::Itertools;
 
/// A Document contains an id and content.
/// Hashing and equality are based only on the id field.
#[derive(Clone, Eq, Debug)]
pub struct Document {
    id: String,
    content: String,
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
struct SearchResult {
    pub doc: Rc<Document>,
    pub highlighted: (usize, usize)
}

/// An Index is anything that can store Documents and return matching Documents for a query.
trait Index {
    /// Given a Document, stores it in the index.
    fn index(&mut self, doc: Document);
    /// Given a query, return the set of Documents that match the query.
    /// What constitutes a "match" is implementation-specific; loosely, a Document should
    /// match a query if they are somehow related. A naive implementation might simply
    /// return the set of Documents for which `query` is a substring of their content.
    fn search(&self, query: &str) -> HashSet<SearchResult>;
}
 
/// A basic implementation of an `Index`, the inverted index is a data structure that maps
/// from words to sets of Documents.
type InvertedIndex = BTreeMap<String, HashSet<SearchResult>>;
 
impl Index for InvertedIndex {
    /// A basic implementation of index, splits the document's content into whitespace-separated
    /// words, and inserts each word-document pair into the map.
    fn index(&mut self, doc: Document) {
        let doc = Rc::new(doc);
        let analyzed = doc.content()
            .char_indices()
            .group_by(|&(_, c)| c.is_whitespace())
            .filter(|&(is_whitespace, _)| !is_whitespace)
            .flat_map(|(_, chars)| (1..chars.len() + 1).map(move |to| {
                let word: String = chars[..to].iter().map(|&(_, c)| c).collect();
                let start = chars[0].0;
                let (last_idx, last_char) = chars[to - 1];
                let finish = last_idx + last_char.len_utf8();
                (word, (start, finish))
            }));

        for (ngram, highlighted) in analyzed {
            self.entry(ngram).or_insert_with(|| HashSet::new())
                .insert(SearchResult { doc: doc.clone(), highlighted: highlighted });
        }
    }
 
    /// A basic search implementation that splits the query's content into whitespace-separated
    /// words, looks up the set of Documents for each word, and then concatenates the sets.
    fn search(&self, query: &str) -> HashSet<SearchResult> {
        query.split_whitespace()
            .flat_map(|word| self.get(word))
            .flat_map(|docs| docs)
            .cloned()
            .collect()
    }
}

#[test]
fn test_search1() {
    let mut index = InvertedIndex::new();    
    let doc1 = Document::new("1", "learn to program in rust today");
    let doc2 = Document::new("2", "what did you today do");
    index.index(doc1.clone());
    index.index(doc2.clone());
    let search_results = index.search("today");
    assert!(search_results.contains(&SearchResult { doc: Rc::new(doc1), highlighted: (25, 30) }));
    assert!(search_results.contains(&SearchResult { doc: Rc::new(doc2), highlighted: (13, 18) }));
}

#[test]
fn test_ngrams() {
    let mut index = InvertedIndex::new();    
    let doc1 = Document::new("1", "learn to program in rust today");
    let doc2 = Document::new("2", "what did you today do");
    index.index(doc1.clone());
    index.index(doc2.clone());
    let search_results = index.search("to");
    println!("{:?}", search_results);
    assert!(search_results.contains(&SearchResult { doc: Rc::new(doc1), highlighted: (25, 27) }));
    assert!(search_results.contains(&SearchResult { doc: Rc::new(doc2), highlighted: (13, 15) }));
}

#[test]
fn test_search2() {
    let mut index = InvertedIndex::new();    
    index.index(Document::new("2", "what to do today"));
    index.index(Document::new("3", "hey today"));
    let search_results = index.search("to");
    assert_eq!(search_results.len(), 3);
}

#[test]
fn test_unicode() {
    let mut index = InvertedIndex::new();    
    let doc = Document::new("0", "abc åäö");
    index.index(doc.clone());
    let to_search = "å";
    let result = index.search(to_search).into_iter().next().unwrap();
    let (begin, end) = result.highlighted;
    assert_eq!(&result.doc.content()[begin..end], to_search);
}

