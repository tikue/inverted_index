use super::Document;

/// A SearchResult is the representation of a Document returned for a specific set of search
/// terms. It is unique upon the document and the vec of highlight indices. It also contains a
/// search score for use in ranking against the other search results
#[derive(Clone, Debug, RustcEncodable)]
pub struct SearchResult<'a> {
    /// The document returned for the search
    pub doc: &'a Document,
    /// The indices of the terms in the document that matched the search
    pub highlights: Vec<(usize, usize)>,
    /// The search score, for use in ranking documents
    pub score: f32,
}

impl<'a> SearchResult<'a> {
    /// Constructs a new SearchResult from the given Document and highlights.
    /// Computes the score using the highlights and the document length
    pub fn new(doc: &'a Document, highlights: Vec<(usize, usize)>) -> SearchResult<'a> {
        SearchResult {
            score: highlights.iter()
                .map(|&(begin, end)| end - begin)
                .sum::<usize>() as f32 / (doc.content().len() as f32).sqrt(),
            doc: doc,
            highlights: highlights
        }
    }

    /// Returns the document
    pub fn doc(&self) -> &Document {
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

    /// Returns the search result's content, surrounding all highlighted terms with `before`
    /// and `after` 
    pub fn highlight(&self, before: &str, after: &str) -> String {
        let mut begin_idx = 0;
        let mut parts = String::new();
        for &(begin, end) in &self.highlights {
            parts.push_str(&self.doc.content()[begin_idx..begin]);
            parts.push_str(before);
            parts.push_str(&self.doc.content()[begin..end]);
            parts.push_str(after);
            begin_idx = end;
        }
        parts.push_str(&self.doc.content()[begin_idx..]);
        parts
    }
}


