//! The hunt list: many words hunted at once, next to the main pattern.
//!
//! Every word is looked for at both ends of the address. The first address that starts with a
//! word is kept, and the first that ends with it; after that the word stops being looked for at
//! that end, so a short word cannot flood the vault.
//!
//! Checking each key against every word would get slower with each word added. Instead the words
//! are packed into one table (`Packed`) with a small bitmap over the three characters nearest
//! each end. Nearly every key fails the bitmap after two lookups; only the rare key that passes
//! is compared with the few words sharing those three characters. The processor uses the table
//! through `Packed::hit`, and the graphics card gets the same table and does the same steps in
//! `kernel/grind.cl`. `Words::take` then confirms each find by plain comparison.

use crate::pattern::{self, Pattern};

pub const MIN_LEN: usize = 3;
pub const MAX_WORDS: usize = 5000;

// The table layout, mirrored in grind.cl.
const BITMAP: usize = 8192;
const START_BITS: usize = 128;
const END_BITS: usize = START_BITS + BITMAP;
const ENTRIES: usize = END_BITS + BITMAP;
const CODE: u32 = 0x3FFFF;
/// The table's full size, which is how big the graphics card's copy is.
pub const PACKED_LEN: usize = ENTRIES + 4 * 2 * MAX_WORDS;

struct Word {
    text: Vec<u8>,
    /// Still looked for at the start / at the end.
    start: bool,
    end: bool,
}

pub struct Words {
    any_case: bool,
    words: Vec<Word>,
}

/// What an address did for the list.
pub struct Taken {
    /// Length of the longest word it starts with and ends with.
    pub start_len: usize,
    pub end_len: usize,
    /// True when it is the first address for at least one word and end.
    pub fresh: bool,
}

/// The 6-bit code of a character. With any case, both forms of a letter get the same code.
fn code(c: u8, any_case: bool) -> u32 {
    let lower = c.to_ascii_lowercase();
    let pick = if any_case && pattern::index(lower).is_some() { lower } else if any_case { c.to_ascii_uppercase() } else { c };
    pattern::index(pick).map_or(63, |i| i as u32)
}

/// One table entry: the characters, five per number and six bits each, then the masks.
fn entry(codes: impl Iterator<Item = u32>) -> [u32; 4] {
    let mut e = [0u32; 4];
    for (i, c) in codes.enumerate() {
        let (slot, shift) = (i / 5, 6 * (i % 5) as u32);
        e[slot] |= c << shift;
        e[2 + slot] |= 63 << shift;
    }
    e
}

impl Words {
    /// Words separated by spaces, commas or new lines.
    pub fn parse(text: &str, any_case: bool) -> Result<Self, String> {
        let mut words: Vec<Word> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for token in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';').filter(|t| !t.is_empty()) {
            let shown: String = token.chars().take(16).collect();
            let bytes = pattern::check(token, any_case).map_err(|e| format!("Hunt list, \"{shown}\": {e}"))?;
            if bytes.len() < MIN_LEN {
                return Err(format!("Hunt list, \"{shown}\": words need at least {MIN_LEN} characters."));
            }
            let key: Vec<u32> = bytes.iter().map(|&c| code(c, any_case)).collect();
            if seen.insert(key) {
                words.push(Word { text: bytes, start: true, end: true });
            }
        }
        if words.len() > MAX_WORDS {
            return Err(format!("The hunt list can hold {MAX_WORDS} words; this one has {}.", words.len()));
        }
        Ok(Self { any_case, words })
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// True when every word has been found at both ends.
    pub fn done(&self) -> bool {
        self.words.iter().all(|w| !w.start && !w.end)
    }

    /// Average number of keys between finds, for the words still being looked for.
    pub fn mean_tries(&self) -> f64 {
        let chance = |start: &[u8], end: &[u8]| {
            1.0 / Pattern { start: start.to_vec(), end: end.to_vec(), any_case: self.any_case }.mean_tries()
        };
        let p: f64 = self
            .words
            .iter()
            .map(|w| if w.start { chance(&w.text, &[]) } else { 0.0 } + if w.end { chance(&[], &w.text) } else { 0.0 })
            .sum();
        if p > 0.0 {
            1.0 / p
        } else {
            0.0
        }
    }

    /// Checks an address against every word by plain comparison and ticks off what it found.
    /// None means it fits no word at all.
    pub fn take(&mut self, addr: &[u8]) -> Option<Taken> {
        let mut t = Taken { start_len: 0, end_len: 0, fresh: false };
        let mut any = false;
        let same = |a: &[u8], b: &[u8]| if self.any_case { a.eq_ignore_ascii_case(b) } else { a == b };
        for w in &mut self.words {
            let n = w.text.len();
            if addr.len() < n {
                continue;
            }
            if same(&addr[..n], &w.text) {
                any = true;
                t.start_len = t.start_len.max(n);
                t.fresh |= std::mem::take(&mut w.start);
            }
            if same(&addr[addr.len() - n..], &w.text) {
                any = true;
                t.end_len = t.end_len.max(n);
                t.fresh |= std::mem::take(&mut w.end);
            }
        }
        any.then_some(t)
    }

    /// The lookup table for the words still being looked for.
    pub fn pack(&self) -> Packed {
        let mut data = vec![0u32; ENTRIES];
        for c in 0..128u8 {
            data[c as usize] = code(c, self.any_case);
        }
        let codes = |w: &Word| w.text.iter().map(|&c| code(c, self.any_case)).collect::<Vec<_>>();
        let mut starts: Vec<[u32; 4]> = self.words.iter().filter(|w| w.start).map(|w| entry(codes(w).into_iter())).collect();
        let mut ends: Vec<[u32; 4]> = self.words.iter().filter(|w| w.end).map(|w| entry(codes(w).into_iter().rev())).collect();
        for (list, bits) in [(&mut starts, START_BITS), (&mut ends, END_BITS)] {
            list.sort_by_key(|e| e[0] & CODE);
            for e in list.iter() {
                let c = (e[0] & CODE) as usize;
                data[bits + c / 32] |= 1 << (c % 32);
            }
        }
        let (starts_n, ends_n) = (starts.len(), ends.len());
        data.extend(starts.into_iter().chain(ends).flatten());
        Packed { data, starts: starts_n, ends: ends_n }
    }
}

pub struct Packed {
    pub data: Vec<u32>,
    pub starts: usize,
    pub ends: usize,
}

impl Packed {
    /// True when the address starts or ends with a word that is still being looked for.
    /// The same steps as `list_any` in grind.cl.
    pub fn hit(&self, addr: &[u8]) -> bool {
        let len = addr.len();
        if len < 10 {
            return false;
        }
        let d = &self.data;
        let at = |i: usize| d[(addr[i] & 127) as usize];
        if self.starts > 0 {
            let c = at(0) | at(1) << 6 | at(2) << 12;
            if d[START_BITS + (c >> 5) as usize] >> (c & 31) & 1 == 1 && self.side(c, ENTRIES, self.starts, |i| at(i)) {
                return true;
            }
        }
        if self.ends > 0 {
            let c = at(len - 1) | at(len - 2) << 6 | at(len - 3) << 12;
            if d[END_BITS + (c >> 5) as usize] >> (c & 31) & 1 == 1 && self.side(c, ENTRIES + 4 * self.starts, self.ends, |i| at(len - 1 - i)) {
                return true;
            }
        }
        false
    }

    /// `nth(i)` is the code of the i-th character counted from the end being checked.
    fn side(&self, code: u32, base: usize, n: usize, nth: impl Fn(usize) -> u32) -> bool {
        let entries = &self.data[base..base + 4 * n];
        let (mut i, mut hi) = (0, n);
        while i < hi {
            let mid = (i + hi) / 2;
            if entries[4 * mid] & CODE < code {
                i = mid + 1;
            } else {
                hi = mid;
            }
        }
        let (mut a0, mut a1) = (0u32, 0u32);
        for k in 0..5 {
            a0 |= nth(k) << (6 * k);
            a1 |= nth(5 + k) << (6 * k);
        }
        while i < n {
            let e = &entries[4 * i..4 * i + 4];
            if e[0] & CODE != code {
                break;
            }
            if ((a0 ^ e[0]) & e[2]) | ((a1 ^ e[1]) & e[3]) == 0 {
                return true;
            }
            i += 1;
        }
        false
    }
}
