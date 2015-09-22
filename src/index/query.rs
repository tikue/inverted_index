/// A Query organizes a search of an inverted index.
/// It is recursively hierarchical, allowing flexibility
/// in exactly how a search is specified
#[derive(Debug)]
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
}
