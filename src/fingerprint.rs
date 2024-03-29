use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use rustc_hash::FxHasher;

pub struct Fingerprint {
    pub spanned_hashes: Vec<(u64, Range<usize>)>,
}

/// Generates a `Fingerprint` for the given list of tokens using the winnowing algorithm.
/// Tokens can be any type that implements the `Hash` trait (chars, tokens from a lexer, etc.).
///
/// Substrings with length at least `t` are guaranteed to be captured in the fingerprint.
/// Substrings with length less than `k` are excluded from the fingerprint.
///
/// If the same hash occurs multiple times in a row, it will only be returned once.
///
/// # Arguments
///
/// * `k` - The noise threshold
/// * `t` - The guarantee threshold
/// * `m` - The maximum value for the offset in relative tokens
///
/// # Panics
///
/// * Panics if `t < k + m`
/// * Panics if `k == 0`
#[inline]
pub fn fingerprint<T>(
    k: usize,
    t: usize,
    m: usize,
    tokens: &[(T, Range<usize>)],
) -> anyhow::Result<Fingerprint>
where
    T: Hash,
{
    assert!(t >= k + m);
    assert!(k != 0);

    let num_tokens = tokens.len();
    if num_tokens < k {
        anyhow::bail!("File could not be fingerprinted because it contains {num_tokens} tokens, which is less than the noise threshold of {k}.");
    }

    // ORIGINAL FORMULA:
    //   The window size is set to t - k + 1 such that at least one hash is
    //   picked from every sequence of hashes of length greater than t - k.
    //
    // GENERALIZATION FOR RELATIVE TOKENS:
    //   Suppose also that two projects have matching code snippets with a
    //   length of t tokens. Then the last t - m tokens will definitely be the
    //   same using the relative lexing scheme (because the relative tokens'
    //   offsets are limited to m and these code snippets are assumed to be
    //   identical). So by choosing the window size to be (t - m) - k + 1, we
    //   can make the same guarantee as with the original formula.
    let w = t - m - k + 1;

    // Generate the hashes of all valid k-grams in the document.
    // By hashing k-grams, we guarantee that no match shorter than k will be included in the
    // fingerprint.
    let hashes = tokens
        .windows(k)
        .map(|w| hash_window(w))
        .collect::<Vec<_>>();

    let fingerprint = choose_fingerprint(&hashes, w);
    Ok(fingerprint)
}

#[inline]
fn hash_window<T>(spanned_tokens: &[(T, Range<usize>)]) -> (u64, Range<usize>)
where
    T: Hash,
{
    // IMPORTANT: create a new hasher each time because hasher.finish() does NOT
    // clear the hasher, it only returns the hash.
    let mut hasher = FxHasher::default();

    let tokens = spanned_tokens.iter().map(|(token, _)| token);

    for token in tokens {
        token.hash(&mut hasher);
    }

    let hash = hasher.finish();

    let spans = spanned_tokens.iter().map(|(_, span)| span.clone());

    let combined_span = combine_spans(spans);

    (hash, combined_span)
}

#[inline]
fn combine_spans(mut spans: impl Iterator<Item = Range<usize>>) -> Range<usize> {
    // Safe to unwrap since this function is only called with non-empty iterators.
    let mut result = spans.next().unwrap();

    for span in spans {
        result.end = span.end;
    }

    result
}

#[inline]
fn choose_fingerprint(spanned_hashes: &[(u64, Range<usize>)], w: usize) -> Fingerprint {
    let mut fingerprint_hashes = vec![];
    let mut previously_picked_hash: Option<u64> = None;

    for window in spanned_hashes.windows(w) {
        let (min_hash, min_hash_span) = window.iter().min_by_key(|(hash, _)| hash).unwrap();
        let min_hash = *min_hash;

        match previously_picked_hash {
            Some(previously_picked_hash) if previously_picked_hash == min_hash => {
                // Do nothing. There's no point in storing the same hash twice in the fingerprint.
            }
            _ => {
                previously_picked_hash = Some(min_hash);
                fingerprint_hashes.push((min_hash, min_hash_span.clone()));
            }
        }
    }

    Fingerprint {
        spanned_hashes: fingerprint_hashes,
    }
}

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    #[test]
    fn moss_example() {
        // Example from page 4 of the MOSS paper adapted for robust winnowing
        // (as well as removing identical back-to-back hashes)
        let hashes = vec![
            (77, 0..1),
            (74, 1..2),
            (42, 2..3),
            (17, 3..4),
            (98, 4..5),
            (50, 5..6),
            (17, 6..7),
            (98, 7..8),
            (8, 8..9),
            (88, 9..10),
            (67, 10..11),
            (39, 11..12),
            (77, 12..13),
            (74, 13..14),
            (42, 14..15),
            (17, 15..16),
            (98, 16..17),
        ];
        let w = 4;
        let fingerprint = choose_fingerprint(&hashes, w);
        assert_eq!(
            fingerprint.spanned_hashes,
            vec![(17, 3..4), (8, 8..9), (39, 11..12), (17, 15..16)]
        );
    }

    #[test]
    fn identical_hashes() {
        let hashes = vec![(1, 0..1), (1, 1..2), (1, 2..3), (1, 3..4), (1, 4..5)];
        let w = 2;
        let fingerprint = choose_fingerprint(&hashes, w);
        assert_eq!(fingerprint.spanned_hashes, vec![(1, 0..1)]);
    }
}
