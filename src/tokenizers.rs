// Original authorship BurntSushi

use std::ascii::AsciiExt;
use std::io;

use util::char_utf8::decode_utf8;

/// A token represents a single atomic unit of information present in a document.
#[derive(Clone, Debug, Hash, Eq, Ord, PartialEq, PartialOrd, RustcDecodable, RustcEncodable)]
pub struct Token {
    /// A single parsed input from the document, possibly after undergoing some series of
    /// transformations.
    pub token: String,
    /// Information about the position of the token within the document.
    pub position: Position,
}

impl Token {
    /// Creates a new token from a string, offsets, and position.
    pub fn new<S: Into<String>>(token: S, offsets: (usize, usize), position: usize) -> Token {
        Token {
            token: token.into(),
            position: Position::new(offsets, position),
        }
    }

    /// Creates an empty token with capacity reserved for 5 bytes;
    pub fn empty() -> Token {
        Token::new(String::with_capacity(5), (0, 0), 0)
    }
}

/// Information about the position of a single term within a document
#[derive(Copy, Clone, Debug, Hash, Eq, Ord, PartialEq, PartialOrd, RustcDecodable, RustcEncodable)]
pub struct Position {
    /// Pair of byte indexes into the document at the beginning (inclusive) and end (exclusive) of 
    /// the term.
    pub offsets: (usize, usize),
    /// The token position of the term, i.e., the number of tokens that occur before it in the doc.
    /// For example, for the sentence "I have to go to the store",
    /// the term "to" has positions [2, 4].
    pub position: usize,
}

impl Position {
    /// Creates a new Position struct with the given offsets and position.
    pub fn new(offsets: (usize, usize), position: usize) -> Position {
        Position {
            offsets: offsets,
            position: position,
        }
    }
}

/// A type that can output a sequence of tokens
pub trait Tokenizer {
    /// Returns the next token read from the input.
    fn read(&mut self, tok: &mut Token) -> io::Result<bool>;

    /// Returns the tokenizer output as an iterator.
    fn into_iter(self) -> Iter<Self> where Self: Sized {
        Iter { tokenizer: self, err: false }
    }
}

/// Iterator over a tokenizer's output.
pub struct Iter<Tknzr> {
    tokenizer: Tknzr,
    err: bool,
}

impl<Tknzr: Tokenizer> Iterator for Iter<Tknzr> {
    type Item = io::Result<Token>;

    fn next(&mut self) -> Option<io::Result<Token>> {
        if self.err {
            return None;
        }
        let mut tok = Token::empty();
        match self.tokenizer.read(&mut tok) {
            Ok(true) => Some(Ok(tok)),
            Ok(false) => None,
            Err(err) => { self.err = true; Some(Err(err)) },
        }
    }
}

/// A tokenizer of english documents encoded in UTF-8.
pub struct EnglishUtf8<Buf> {
    rdr: Buf,
    offset: usize,
    num_tokens: usize,
}

impl<Buf: io::BufRead> EnglishUtf8<Buf> {
    /// Creates a new tokenizer backed by the given buffer.
    pub fn new(rdr: Buf) -> EnglishUtf8<Buf> {
        EnglishUtf8 { 
            rdr: rdr,
            offset: 0,
            num_tokens: 0,
        }
    }
}

impl EnglishUtf8<io::Cursor<Vec<u8>>> {
    /// Construct an EnglishUtf8 tokenizer backed by a byte buffer.
    pub fn from_bytes<B>(bytes: B) -> EnglishUtf8<io::Cursor<Vec<u8>>>
        where B: Into<Vec<u8>> 
    {
        EnglishUtf8::new(io::Cursor::new(bytes.into()))
    }

    /// Reset the backing buffer to position 0.
    pub fn reset(&mut self) {
        self.rdr.set_position(0);
    }
}

impl<Buf: io::BufRead> Tokenizer for EnglishUtf8<Buf> {
    fn read(&mut self, tok: &mut Token) -> io::Result<bool> {
        let mut consumed = 0;
        tok.token.clear();
'LOOP:  loop {
            self.rdr.consume(consumed);
            consumed = 0;
            let buf = try!(self.rdr.fill_buf());
            if buf.is_empty() {
                if tok.token.is_empty() {
                    return Ok(false);
                } else {
                    break 'LOOP;
                }
            }
            while consumed < buf.len() {
                let bytes = &buf[consumed..];
                let (n, c) = match decode_utf8(bytes) {
                    None => {
                        consumed += 1;
                        self.offset += 1;
                        continue
                    }
                    Some((n, c)) => { consumed += n; (n, c) }
                };
                if c.is_whitespace() {
                    self.offset += n;
                    if tok.token.is_empty() {
                        continue;
                    } else {
                        break 'LOOP;
                    }
                }
                if !c.is_alphanumeric() {
                    self.offset += n;
                    continue;
                }
                let c = c.to_ascii_lowercase();
                if tok.token.is_empty() {
                    tok.position.offsets.0 = self.offset;
                }
                self.offset += n;
                tok.token.push(c);
                tok.position.offsets.1 = self.offset;
            }
        }
        self.rdr.consume(consumed);
        tok.position.position = self.num_tokens;
        self.num_tokens += 1;
        Ok(true)
    }
}

/// An analyzer that tokenizes its input and returns each subslice of each token that starts from
/// the first char.
pub struct NgramsAnalyzer<Tknzr: Tokenizer> {
    tokenizer: Tknzr,
    next: Vec<Token>,
}

impl<Tknzr: Tokenizer> Tokenizer for NgramsAnalyzer<Tknzr> {
    fn read(&mut self, tok: &mut Token) -> io::Result<bool> {
        match self.next.pop() {
            Some(next) => { 
                *tok = next;
                Ok(true)
            }
            None => {
                match self.tokenizer.read(tok) {
                    done @ Ok(false) | Err(_) => done,
                    Ok(true) => {
                        let chars: Vec<_> = tok.token.char_indices().collect();
                        (1..chars.len() + 1).map(|to| {
                            let word = chars[..to]
                                           .iter()
                                           .flat_map(|&(_, c)| c)
                                           .collect();
                            let start = self.chars[0].0;
                            let (last_idx, last_char) = self.chars[to - 1];
                            let finish = last_idx + last_char.len_utf8();
                            (word, Position::new((start, finish), self.position))
                    }
                }
            }
        }
    }

    type TokenPositions = iter::FlatMap<
                    iter::Enumerate<WordPositionsNoWhitespace<'a>>,
                    NgramsIter,
                    NgramsFn>;

    fn analyze(&self, s: &'a str) -> Self::TokenPositions {

        s.char_indices()
         .group_by(is_whitespace as fn(&(usize, char)) -> bool)
         .filter(not_whitespace as fn(&(bool, Vec<(usize, char)>)) -> bool)
         .enumerate()
         .flat_map(ngrams)
    }
}

type WordPositions<'a> = GroupBy<bool, CharIndices<'a>, fn(&(usize, char)) -> bool>;
type WordPositionsNoWhitespace<'a> = iter::Filter<
    WordPositions<'a>,
    fn(&(bool, Vec<(usize, char)>)) -> bool>;
type NgramsIter = iter::Map<ops::Range<usize>, Ngrams>;
type NgramsFn = fn((usize, (bool, Vec<(usize, char)>))) -> NgramsIter;
type OnlyTokens<I> = iter::Map<I, fn((String, Position)) -> String>;
type WhitespaceTokenPositions<'a> = iter::Map<iter::Enumerate<WordPositionsNoWhitespace<'a>>, 
                        fn((usize, (bool, Vec<(usize, char)>))) -> (String, Position)>;

fn not_whitespace(&(is_whitespace, _): &(bool, Vec<(usize, char)>)) -> bool {
    !is_whitespace
}

fn is_whitespace(&(_, c): &(usize, char)) -> bool {
    c.is_whitespace()
}

struct Ngrams {
    position: usize,
    chars: Vec<(usize, char)>,
}

impl Ngrams {
    fn new(position: usize, chars: Vec<(usize, char)>) -> Ngrams {
        Ngrams {
            position: position,
            chars: chars,
        }
    }
}

impl Fn<(usize,)> for Ngrams {
    extern "rust-call" fn call(&self, (to,): (usize,)) -> (String, Position) {
        let word = self.chars[..to].iter().flat_map(|&(_, c)| c.to_lowercase()).collect();
        let start = self.chars[0].0;
        let (last_idx, last_char) = self.chars[to - 1];
        let finish = last_idx + last_char.len_utf8();
        (word, Position::new((start, finish), self.position))
    }
}

impl FnMut<(usize,)> for Ngrams {
    extern "rust-call" fn call_mut(&mut self, to: (usize,)) -> (String, Position) {
        self.call(to)
    }
}

impl FnOnce<(usize,)> for Ngrams {
    type Output = (String, Position);
    extern "rust-call" fn call_once(self, to: (usize,)) -> (String, Position) {
        self.call(to)
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use super::{Tokenizer, Token, EnglishUtf8};

    fn collect<T: Tokenizer>(tokenizer: T) -> Vec<Token> {
        tokenizer.into_iter().collect::<Result<Vec<_>, _>>().unwrap()
    }

    #[test]
    fn tiny_buffer() {
        let bytes = &b"Hi, Dave! How are you?"[..];
        let buf = io::BufReader::with_capacity(1, bytes);
        let toks = collect(EnglishUtf8::new(buf));
        assert_eq!(toks, vec![
                   Token::new("hi", (0, 2), 0),
                   Token::new("dave", (4, 8), 1),
                   Token::new("how", (10, 13), 2),
                   Token::new("are", (14, 17), 3),
                   Token::new("you", (18, 21), 4)
        ]);
    }
}
