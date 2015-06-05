extern crate itertools;

use std::collections::{BTreeMap, HashMap, HashSet};
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
struct IndexedDocument {
    pub doc: Rc<Document>,
    pub highlighted: (usize, usize),
}

/// An Index is anything that can store Documents and return matching Documents for a query.
pub trait Index {
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
pub type InvertedIndex = BTreeMap<String, HashSet<IndexedDocument>>;
 
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
                .insert(IndexedDocument { doc: doc.clone(), highlighted: highlighted });
        }
    }
 
    /// A basic search implementation that splits the query's content into whitespace-separated
    /// words, looks up the set of Documents for each word, and then concatenates the sets.
    fn search(&self, query: &str) -> HashSet<SearchResult> {
        let map = query.split_whitespace()
            .flat_map(|word| self.get(word))
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
    doc: Rc<Document>,
    highlights: Vec<(usize, usize)>,
}

impl SearchResult {
    #[cfg(test)]
    fn new(doc: Rc<Document>, highlights: Vec<(usize, usize)>) -> SearchResult {
        SearchResult {
            doc: doc,
            highlights: highlights,
        }
    }

    pub fn doc(&self) -> &Rc<Document> {
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
        SearchResult::new(Rc::new(doc1), vec![(25, 30)]),
        SearchResult::new(Rc::new(doc2), vec![(13, 18)])
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
        SearchResult::new(Rc::new(doc1), vec![(6, 8), (25, 27)]),
        SearchResult::new(Rc::new(doc2), vec![(13, 15)]),
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
        SearchResult::new(Rc::new(doc1), vec![(5, 7), (11, 13)]),
        SearchResult::new(Rc::new(doc2), vec![(4, 6)]),
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
