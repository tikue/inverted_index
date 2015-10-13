/// A Query organizes a search of an inverted index.
/// It is recursively hierarchical, allowing flexibility
/// in exactly how a search is specified
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, RustcEncodable)]
pub enum Query<'a> {
    /// The simplest query, represents a search using
    /// the given string
    Match(&'a str),

    /// A query requesting the intersection of the documents 
    /// returned in each sub-query
    And(&'a [Query<'a>]),

    /// A query requesting the union of the documents returned
    /// in each sub-query
    Or(&'a [Query<'a>]),

    /// An exact-match query. The given phrase must appear in all documents returned.
    /// False positives may occur.
    Phrase(&'a str),

    /// A prefix query that returns all documents containing terms with the given prefix.
    /// Note that, unlike `Match` and `Phrase`, this query is not tokenized before searching
    /// the index. Thus, Prefix("hi bob") is likely to match zero documents, since indexed
    /// documents typically have their content tokenized upon spaces.
    Prefix(&'a str),
}
