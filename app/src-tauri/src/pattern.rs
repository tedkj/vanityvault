//! Pattern rules and the difficulty estimate.
//!
//! A Solana address is the base58 form of a 32-byte key, which is 43 or 44 characters long. Because
//! 2^256 is not a power of 58, the first character is far from uniform: a 44-character address can
//! only start with a low character ('2'..'J'), and every other first character needs the rarer
//! 43-character form. The
//! other positions are close enough to uniform.

pub const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
/// Longest allowed start or end, each.
pub const MAX_LEN: usize = 10;

/// What to look for: a start, an end, or both at once.
#[derive(Clone, Debug)]
pub struct Pattern {
    pub start: Vec<u8>,
    pub end: Vec<u8>,
    pub any_case: bool,
}

pub(crate) fn index(c: u8) -> Option<usize> {
    ALPHABET.iter().position(|&a| a == c)
}

/// The base58 characters that count as a match for `c`.
fn variants(c: u8, any_case: bool) -> Vec<u8> {
    if any_case && c.is_ascii_alphabetic() {
        let mut v: Vec<u8> = [c.to_ascii_lowercase(), c.to_ascii_uppercase()]
            .into_iter()
            .filter(|&x| index(x).is_some())
            .collect();
        v.dedup();
        v
    } else if index(c).is_some() {
        vec![c]
    } else {
        Vec::new()
    }
}

pub(crate) fn check(text: &str, any_case: bool) -> Result<Vec<u8>, String> {
    if text.chars().count() > MAX_LEN {
        return Err(format!("Keep each part to {MAX_LEN} characters or fewer."));
    }
    for ch in text.chars() {
        let ok = ch.is_ascii() && !variants(ch as u8, any_case).is_empty();
        if !ok {
            return Err(match ch {
                '0' | 'O' | 'I' | 'l' => {
                    format!("\"{ch}\" never appears in a Solana address. The four missing characters are 0, O, I and l.")
                }
                _ => format!("\"{ch}\" can't appear in a Solana address. Use letters and digits only."),
            });
        }
    }
    Ok(text.as_bytes().to_vec())
}

impl Pattern {
    pub fn parse(start: &str, end: &str, any_case: bool) -> Result<Self, String> {
        if start.is_empty() && end.is_empty() {
            return Err("Type at least one character.".into());
        }
        Ok(Self { start: check(start, any_case)?, end: check(end, any_case)?, any_case })
    }

    pub fn matches(&self, addr: &[u8]) -> bool {
        let (s, e) = (self.start.len(), self.end.len());
        if addr.len() < s + e {
            return false;
        }
        let (head, tail) = (&addr[..s], &addr[addr.len() - e..]);
        if self.any_case {
            head.eq_ignore_ascii_case(&self.start) && tail.eq_ignore_ascii_case(&self.end)
        } else {
            head == &self.start[..] && tail == &self.end[..]
        }
    }

    /// Average number of keys to try before one matches.
    pub fn mean_tries(&self) -> f64 {
        let mut p = 1.0f64;
        for (i, &c) in self.start.iter().enumerate() {
            let vs = variants(c, self.any_case);
            if i == 0 {
                p *= vs.iter().map(|&v| p_first(v)).sum::<f64>();
            } else {
                p *= vs.len() as f64 / 58.0;
            }
        }
        for &c in &self.end {
            p *= variants(c, self.any_case).len() as f64 / 58.0;
        }
        if p > 0.0 {
            1.0 / p
        } else {
            f64::INFINITY
        }
    }
}

/// Chance that a random 32-byte key's address starts with `c`.
fn p_first(c: u8) -> f64 {
    let Some(idx) = index(c) else { return 0.0 };
    let n256 = 2f64.powi(256);
    let mut total = 0.0;
    for len in [43, 44] {
        let unit = 58f64.powi(len - 1);
        let lo = (idx as f64 * unit).max(unit);
        let hi = ((idx as f64 + 1.0) * unit).min(58f64.powi(len)).min(n256);
        if hi > lo {
            total += hi - lo;
        }
    }
    total / n256
}
