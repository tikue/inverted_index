/// A Document contains an id and content.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, RustcEncodable, RustcDecodable)]
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
        Document {
            id: id.into(),
            content: content.into(),
        }
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
