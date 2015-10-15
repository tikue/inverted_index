use std::char;

/// A trait for types whose values have well-defined successors.
pub trait Successor: Sized {
    /// Returns the successor to self, if any exists.
    fn successor(&self) -> Option<Self>;
}

impl Successor for char {
    #[inline]
    // Implementation lifted from https://github.com/huonw/char-iter/blob/master/src/lib.rs#L77
    fn successor(&self) -> Option<char> {
        const SUR_START: u32 = 0xD800;
        const SUR_END: u32 = 0xDFFF;
        const BEFORE_SUR: u32 = SUR_START - 1;
        const AFTER_SUR: u32 = SUR_END + 1;
        let val = *self as u32;
        char::from_u32(if val == BEFORE_SUR {
            AFTER_SUR
        } else {
            val + 1
        })
    }
}
