use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::hash_map::Entry::*;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::iter;
use std::str::CharIndices;
use std::ops;
use itertools::{GroupBy, Itertools};
use coalesce::Coalesce;
 
/// A Document contains an id and content.
/// Hashing and equality are based only on the id field.
#[derive(Clone, Eq, Debug, RustcEncodable, RustcDecodable)]
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
    // Documents are unique only upon their id
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.id.hash(state);
    }
}
 
impl PartialEq<Document> for Document {
    // Documents are unique only upon their id
    fn eq(&self, other: &Document) -> bool {
        self.id == other.id
    }
}

/// A SearchResult is the representation of a Document returned for a specific set of search
/// terms. It is unique upon the document and the vec of highlight indices. It also contains a
/// score and a String of the document's content post-highlighting.
#[derive(Clone, Debug, RustcEncodable)]
pub struct SearchResult {
    doc: Arc<Document>,
    highlights: Vec<(usize, usize)>,
    score: f32,
}

impl PartialEq for SearchResult {
    // SearchResult is unique upon its document and highlights
    fn eq(&self, other: &SearchResult) -> bool {
        self.doc == other.doc && self.highlights == other.highlights
    }
}

impl Eq for SearchResult {}

impl Hash for SearchResult {
    // SearchResult is unique upon its document and highlights
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.doc.hash(state);
        self.highlights.hash(state);
    }
}

impl SearchResult {
    fn new(doc: Arc<Document>, mut highlights: Vec<(usize, usize)>) -> SearchResult {
        SearchResult {
            score: highlights.iter()
                .map(|&(begin, end)| end - begin)
                .sum::<usize>() as f32 / (doc.content.len() as f32).sqrt(),
            doc: doc,
            highlights: { highlights.sort(); highlights },
        }
    }

    /// Returns the document
    pub fn doc(&self) -> &Arc<Document> {
        &self.doc
    }

    /// Returns the highlighted indices.
    ///
    /// Each `(usize, usize)` indicates the start and end of a term in the document's content
    /// that should be highlighted.
    pub fn highlights(&self) -> &Vec<(usize, usize)> {
        &self.highlights
    }

    /// Returns the search result's score.
    ///
    /// Score is computed by the product of the summed length of the matching terms and the inverse
    /// square root of the length of the document. Taking the square root of the document's length
    /// helps to combat bias toward short content.
    pub fn score(&self) -> f32 {
        self.score
    }

    pub fn highlight(&self, before: &str, after: &str) -> String {
        let mut begin_idx = 0;
        let mut parts = String::new();
        for &(begin, end) in &self.highlights {
            parts.push_str(&self.doc.content[begin_idx..begin]);
            parts.push_str(before);
            parts.push_str(&self.doc.content[begin..end]);
            parts.push_str(after);
            begin_idx = end;
        }
        parts.push_str(&self.doc.content[begin_idx..]);
        parts
    }
}

/// A basic implementation of an `Index`, the inverted index is a data structure that maps
/// from words to sets of Documents.
#[derive(Debug)]
pub struct InvertedIndex {
    index: BTreeMap<String, BTreeMap<String, Vec<(usize, usize)>>>,
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

        for (ngram, highlighted) in analyzed {
            let mut highlights = self.index.entry(ngram)
                .or_insert_with(BTreeMap::new)
                .entry(doc.id.clone())
                .or_insert_with(Vec::new);
            let coalesce_idx = highlights.binary_search(&highlighted).err().unwrap();
            highlights.coalesce(coalesce_idx, highlighted);
        }
    }
 
    /// A basic search implementation that splits the query's content into whitespace-separated
    /// words, looks up the set of Documents for each word, and then concatenates the sets.
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let unique_terms: HashSet<_> = query.split_whitespace().map(str::to_lowercase).collect();
        let map: HashMap<String, Vec<(usize, usize)>> = unique_terms.into_iter()
            .flat_map(|word| self.index.get(&word))
            .flat_map(BTreeMap::iter)
            .fold(HashMap::new(), |mut map, (doc_id, highlights)| {
                match map.entry(doc_id.clone()) {
                    Vacant(entry) => { entry.insert(highlights.clone()); },
                    Occupied(mut entry) => {
                        let entry: &mut Vec<(usize, usize)> = entry.get_mut();
                        for highlight in highlights {
                            let coalesce_idx = entry.binary_search(highlight).err().unwrap();
                            entry.coalesce(coalesce_idx, highlight.clone()); 
                        }
                    }
                }
                map
            });
        let mut results: Vec<_> = map.into_iter()
            .map(|(doc_id, index_map)| SearchResult::new(self.docs[&doc_id].clone(), index_map.into_iter().collect()))
            .collect();
        results.sort_by(|result1, result2| result2.score.partial_cmp(&result1.score).unwrap());
        results
    }
}

fn analyze_doc(doc: &str) 
-> iter::FlatMap<
    iter::Filter<
        GroupBy<bool, CharIndices, fn(&(usize, char)) -> bool>, 
        fn(&(bool, Vec<(usize, char)>)) -> bool>, 
    iter::Map<ops::Range<usize>, Ngrams>,
    fn((bool, Vec<(usize, char)>)) -> iter::Map<ops::Range<usize>, Ngrams>> 
{
    doc.char_indices()
        .group_by(is_whitespace as fn(&(usize, char)) -> bool)
        .filter(not_whitespace as fn(&(bool, Vec<(usize, char)>)) -> bool)
        .flat_map(ngrams)
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

#[cfg(test)]
mod bench {
    use super::*;
    use std::sync::Arc;
    use std::collections::HashSet;

    #[test]
    fn test_search1() {
        let mut index = InvertedIndex::new();    
        let doc1 = Document::new("1", "learn to program in rust today");
        index.index(doc1.clone());
        let doc2 = Document::new("2", "what did you today do");
        index.index(doc2.clone());
        let search_results = index.search("to");
        let expected = [
            SearchResult::new(Arc::new(doc1), vec![(6, 8), (25, 27)]),
            SearchResult::new(Arc::new(doc2), vec![(13, 15)])
        ];
        assert_eq!(search_results, expected.iter().cloned().collect::<Vec<_>>());
        assert_eq!("learn <span class=highlight>to</span> program in rust \
                   <span class=highlight>to</span>day", 
                   expected[0].highlight("<span class=highlight>", "</span>"));
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
        assert_eq!(search_results, expected.iter().cloned().collect::<Vec<_>>());
        assert_eq!("learn <span class=highlight>to</span> program in rust \
                   <span class=highlight>to</span>day", 
                   expected[0].highlight("<span class=highlight>", "</span>"));

    }

    #[test]
    fn test_highlight1() {
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
        assert_eq!(search_results.into_iter().collect::<HashSet<_>>(), 
                   expected.iter().cloned().collect());
        assert_eq!("what <span class=highlight>to</span> do <span class=highlight>to</span>day",
                   expected[0].highlight("<span class=highlight>", "</span>"));
    }

    #[test]
    fn test_highlight2() {
        let mut index = InvertedIndex::new();    
        let doc1 = Document::new("2", "Won\u{2019}t this split the ecosystem? Will everyone use?");
        index.index(doc1.clone());
        let expected =  "Won\u{2019}t this split the *e*cosystem? Will *e*veryone use?";
        let search_results = index.search("e");
        assert_eq!(1, search_results.len());
        assert_eq!(search_results[0].highlight("*", "*"), expected);
    }

    #[test]
    fn test_unicode() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "嗨, 您好");
        index.index(doc.clone());
        let to_search = "您";
        let search_results = index.search(to_search);
        let &SearchResult { ref doc, ref highlights, .. } = search_results.iter().next().unwrap();
        let (begin, end) = highlights[0];
        assert_eq!(&doc.content()[begin..end], to_search);
    }

    #[test]
    fn test_update_doc() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "abc åäö");
        index.index(doc);
        let doc = Document::new("0", "different");
        index.index(doc);
        let search_results = index.search("å");
        assert!(search_results.is_empty());
        assert_eq!(index.docs.len(), 1);
    }

    #[test]
    fn test_ranking() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "beat");
        index.index(doc.clone());
        let doc2 = Document::new("1", "beast");
        index.index(doc2.clone());
        let search_results = index.search("be");
        assert_eq!(index.docs.len(), 2);
        assert_eq!(search_results.into_iter().map(|r| r.doc).collect::<HashSet<_>>(),
                [Arc::new(doc), Arc::new(doc2)].iter().cloned().collect());
    }

    #[test]
    fn test_duplicate_term() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "beat");
        index.index(doc.clone());
        let search_results = index.search("be be");
        assert_eq!(search_results.len(), 1);
    }

    #[test]
    fn test_duplicate_term2() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "beat");
        index.index(doc.clone());
        let search_results = index.search("be b");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0], SearchResult::new(Arc::new(doc), vec![(0, 2)]));
    }

    #[test]
    fn test_lowercase_search() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "BeAt");
        index.index(doc.clone());
        let search_results = index.search("bE");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0], SearchResult::new(Arc::new(doc), vec![(0, 2)]));
    }

    #[test]
    fn test_lowercase_index() {
        let mut index = InvertedIndex::new();    
        let doc = Document::new("0", "BeAt");
        index.index(doc.clone());
        let search_results = index.search("be");
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0], SearchResult::new(Arc::new(doc), vec![(0, 2)]));
    }
}
