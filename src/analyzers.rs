use std::iter;
use std::ops;
use std::str::CharIndices;
use itertools::{GroupBy, Itertools};
use tokenizers::{Position, Token};

/// An analyzer turns a string into an iterator of pairs of tokens and positions.
pub trait Analyzer<'a> {
    /// The type of the iterator returned.
    type TokenPositions: Iterator<Item=(String, Position)>;

    /// Tokenizes the input string.
    fn analyze(&self, &'a str) -> Self::TokenPositions;

    /// Tokenizes the input string. Returns only the tokens, without positional information.
    fn analyze_tokens(&self, s: &'a str) -> OnlyTokens<Self::TokenPositions> {
        fn only_token((token, _): (String, Position)) -> String {
            token
        }

        self.analyze(s).map(only_token)
    }
}

/// An analyzer that splits its input on whitespace and lowercases each token.
pub struct WhitespaceAnalyzer;

impl<'a> Analyzer<'a> for WhitespaceAnalyzer {
    type TokenPositions = WhitespaceTokenPositions<'a>;

    fn analyze(&self, s: &'a str) -> Self::TokenPositions {
        fn token_and_pos((position, (_, chars)): (usize, (bool, Vec<(usize, char)>))) 
            -> (String, Position) {
            let len = chars.len();
            Ngrams::new(position, chars)(len)
        }

        s.char_indices()
         .group_by(is_whitespace as fn(&(usize, char)) -> bool)
         .filter(not_whitespace as fn(&(bool, Vec<(usize, char)>)) -> bool)
         .enumerate()
         .map(token_and_pos)
    }
}

/// An analyzer that tokenizes its input and returns each subslice of each token that starts from
/// the first char.
pub struct NgramsAnalyzer;

impl<'a> Analyzer<'a> for NgramsAnalyzer {
    type TokenPositions = iter::FlatMap<
                    iter::Enumerate<WordPositionsNoWhitespace<'a>>,
                    NgramsIter,
                    NgramsFn>;

    fn analyze(&self, s: &'a str) -> Self::TokenPositions {
        fn ngrams((position, (_, chars)): (usize, (bool, Vec<(usize, char)>)))
                  -> iter::Map<ops::Range<usize>, Ngrams> {
            (1..chars.len() + 1).map(Ngrams::new(position, chars))
        }

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

