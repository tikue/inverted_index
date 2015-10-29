// Original authorship burntsushi.
#![cfg_attr(rustfmt, rustfmt_skip)]
use std::str;

// UTF-8 ranges and tags for encoding characters
const TAG_CONT: u8    = 0b1000_0000;
const TAG_TWO_B: u8   = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8  = 0b1111_0000;
const MAX_ONE_B: u32   =     0x80;
const MAX_TWO_B: u32   =    0x800;
const MAX_THREE_B: u32 =  0x10000;

/// Write a utf-8 char as a sequence of bytes.
#[inline]
pub fn encode_utf8(character: char, dst: &mut [u8]) -> Option<usize> {
    let code = character as u32;
    if code < MAX_ONE_B && !dst.is_empty() {
        dst[0] = code as u8;
        Some(1)
    } else if code < MAX_TWO_B && dst.len() >= 2 {
        dst[0] = (code >> 6 & 0x1F) as u8 | TAG_TWO_B;
        dst[1] = (code & 0x3F) as u8 | TAG_CONT;
        Some(2)
    } else if code < MAX_THREE_B && dst.len() >= 3  {
        dst[0] = (code >> 12 & 0x0F) as u8 | TAG_THREE_B;
        dst[1] = (code >>  6 & 0x3F) as u8 | TAG_CONT;
        dst[2] = (code & 0x3F) as u8 | TAG_CONT;
        Some(3)
    } else if dst.len() >= 4 {
        dst[0] = (code >> 18 & 0x07) as u8 | TAG_FOUR_B;
        dst[1] = (code >> 12 & 0x3F) as u8 | TAG_CONT;
        dst[2] = (code >>  6 & 0x3F) as u8 | TAG_CONT;
        dst[3] = (code & 0x3F) as u8 | TAG_CONT;
        Some(4)
    } else {
        None
    }
}

/// Converts a sequence of bytes into a char, if possible. Also returns the byte len of the char.
#[inline]
pub fn decode_utf8(src: &[u8]) -> Option<(usize, char)> {
    if src.len() == 0 {
        return None;
    }
    match utf8_char_width(src[0]) {
        0 => None,
        1 => Some((1, src[0] as char)),
        n if src.len() < n => None,
        n => {
            match str::from_utf8(&src[0..n]) {
                Ok(s) => Some((n, s.chars().next().unwrap())),
                Err(_) => None,
            }
        }
    }
}

// https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 256] = [
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x1F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x3F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x5F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x7F
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0x9F
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0xBF
0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, // 0xDF
3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3, // 0xEF
4,4,4,4,4,0,0,0,0,0,0,0,0,0,0,0, // 0xFF
];

/// Given a first byte, determine how many bytes are in this UTF-8 character
#[inline]
pub fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[b as usize] as usize
}
