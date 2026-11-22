//! The probe values: deterministic, multi-byte, with readable cut points.

/// The three lengths, in chars.
pub(crate) const LEVELS: [(&str, usize); 3] =
    [("control", 40), ("realistic", 420), ("oversize", 5000)];

/// `marker`, then blocks `[offset]„Tarif ä“ ` (offset = char index of the
/// block start, zero-padded), cut at exactly `chars` chars.
pub(crate) fn value(marker: &str, chars: usize) -> String {
    let mut s = format!("{marker} ");
    while s.chars().count() < chars {
        let offset = s.chars().count();
        s.push_str(&format!("[{offset:04}]„Tarif ä“ "));
    }
    s.chars().take(chars).collect()
}

/// Char index of the first difference, `None` when equal.
pub(crate) fn first_diff(sent: &str, got: &str) -> Option<usize> {
    let mut a = sent.chars();
    let mut b = got.chars();
    let mut i = 0;
    loop {
        match (a.next(), b.next()) {
            (None, None) => return None,
            (x, y) if x == y => i += 1,
            _ => return Some(i),
        }
    }
}

/// The last `n` chars of `s`, tabs and newlines flattened for the TSV.
pub(crate) fn tail(s: &str, n: usize) -> String {
    let all: Vec<char> = s.chars().collect();
    all[all.len().saturating_sub(n)..]
        .iter()
        .map(|c| {
            if matches!(c, '\t' | '\n' | '\r') {
                ' '
            } else {
                *c
            }
        })
        .collect()
}
