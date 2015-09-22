use std::hash::{Hash, Hasher};

/// A Document contains an id and content.
/// Hashing and equality are based only on the id field.
#[derive(Clone, Debug, Eq, Ord, RustcEncodable, RustcDecodable)]
pub struct Document {
    /// The id of the document
    pub id: String,
    /// The document's content
    pub content: String,
}

impl Document {
    /// Construct a new Document from an id and content.
    /// Both two arguments can be anything that can be turned into a String.
    pub fn new<S, T>(id: S, content: T) -> Document
        where S: Into<String>,
              T: Into<String>
    {
        Document { id: id.into(), content: content.into() }
    }

    /// Returns a reference to the document's id
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a reference to the document's content
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Hash for Document {
    // Documents are unique only upon their id
    fn hash<H>(&self, state: &mut H)
        where H: Hasher
    {
        self.id.hash(state);
    }
}

impl PartialEq for Document {
    // Documents are unique only upon their id
    fn eq(&self, other: &Document) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Document {
    fn partial_cmp(&self, other: &Document) -> Option<::std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}
