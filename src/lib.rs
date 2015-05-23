extern crate shared_slice;

use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::str::from_utf8;
use shared_slice::rc::RcStr;
 
/// A Document contains an id and content, both of which are reference-counted strings.
/// Hashing and equality are based only on the id field.
#[derive(Clone, Eq)]
pub struct Document {
    pub id: RcStr,
    pub content: RcStr,
}

impl Document {
    /// Construct a new Document from an id and content.
    /// Both two arguments can be anything that can be turned into a String.
    pub fn new<S, T>(id: S, content: T) -> Document
        where S: Into<String>,
              T: Into<String> 
    {
        Document { 
            id: into_rc_str(id),
            content: into_rc_str(content),
        }
    }
}
 
fn into_rc_str<S: Into<String>>(id: S) -> RcStr {
    let id = id.into();
    let id: Vec<u8> = id.into();
    RcStr::new(id.into_boxed_slice())
}

impl Hash for Document {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        state.write(&*self.id);
    }
}
 
impl PartialEq<Document> for Document {
    fn eq(&self, other: &Document) -> bool {
        self.id == other.id
    }
}
 
impl std::fmt::Debug for Document {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "Document {{ id: {:?}, content: {:?} }}", 
               from_utf8(&*self.id).unwrap(), from_utf8(&*self.content).unwrap())
    }
}

/// An Index is anything that can store Documents and return matching Documents for a query.
trait Index {
    /// Given a Document, stores it in the index.
    fn index(&mut self, doc: Document);
    /// Given a query, return the set of Documents that match the query.
    /// What constitutes a "match" is implementation-specific; loosely, a Document should
    /// match a query if they are somehow related. A naive implementation might simply
    /// return the set of Documents for which `query` is a substring of their content.
    fn search(&self, query: &str) -> HashSet<Document>;
}
 
/// A basic implementation of an `Index`, the inverted index is a data structure that maps
/// from words to sets of Documents.
type InvertedIndex = BTreeMap<RcStr, HashSet<Document>>;
 
impl Index for InvertedIndex {
    /// A basic implementation of index, splits the document's content into whitespace-separated
    /// words, and inserts each word-document pair into the map.
    fn index(&mut self, doc: Document) {
        for word in doc.content.clone().split_whitespace() {
            self.entry(word).or_insert_with(|| HashSet::new()).insert(doc.clone());
        }
    }
 
    /// A basic search implementation that splits the query's content into whitespace-separated
    /// words, looks up the set of Documents for each word, and then concatenates the sets.
    fn search(&self, query: &str) -> HashSet<Document> {
        query.split_whitespace()
            .flat_map(|word| self.get(&into_rc_str(word)))
            .flat_map(|docs| docs)
            .cloned()
            .collect()
    }
}

#[test]
fn test_index() {
    let mut index = BTreeMap::new();    

    let doc1 = Document::new("1", "learn to program in rust today");
    index.index(doc1.clone());

    let doc2 = Document::new("2", "what did you do today");
    index.index(doc2.clone());

    let search_results = index.search("today");
    assert!(search_results.contains(&doc1));
    assert!(search_results.contains(&doc2));
    assert_eq!(2, index.search("rust today").len())
}
