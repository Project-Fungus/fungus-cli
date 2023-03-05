use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;

pub struct Fingerprint {
    pub hashes: Vec<u64>,
}

/// Generates a `Fingerprint` for the given vector of tokens using the winnowing algorithm.
/// Tokens can be any type that implements the `Hash` trait (chars, tokens from a lexer, etc.).
///
/// Substrings with length at least `t` are guaranteed to be captured in the fingerprint.
/// Substrings with length less than `k` are excluded from the fingerprint.
///
/// If the same hash occurs multiple times in a row, it will only be returned once.
pub fn fingerprint<T>(k: usize, t: usize, tokens: &[T]) -> Fingerprint
where
    T: Hash,
{
    assert!(k <= t);
    assert!(k != 0);

    // The window size is set to t - k + 1 such that at least one hash is picked from every
    // sequence of hash of length greater than t - k.
    let w = t - k + 1;

    // Generate the hashes of all valid k-grams in the document.
    // By hashing k-grams, we guarantee that no match shorter than k will be included in the
    // fingerprint.
    let hashes = tokens
        .windows(k)
        .map(|w| hash_window(w))
        .collect::<Vec<_>>();

    choose_fingerprint(&hashes, w)
}

#[inline]
fn hash_window<T>(tokens: &[T]) -> u64
where
    T: Hash,
{
    // IMPORTANT: create a new hasher each time because hasher.finish() does NOT
    // clear the hasher, it only returns the hash.
    let mut hasher = FxHasher::default();
    tokens.hash(&mut hasher);
    hasher.finish()
}

fn choose_fingerprint(hashes: &[u64], w: usize) -> Fingerprint {
    let mut fingerprint_hashes = vec![];
    let mut previously_picked_hash = None;

    for window in hashes.windows(w) {
        let &min_hash = window.iter().min().unwrap();

        match previously_picked_hash {
            Some(previously_picked_hash) if previously_picked_hash == min_hash => {
                // Do nothing. There's no point in storing the same hash twice in the fingerprint.
            }
            _ => {
                previously_picked_hash = Some(min_hash);
                fingerprint_hashes.push(min_hash);
            }
        }
    }

    Fingerprint {
        hashes: fingerprint_hashes,
    }
}

#[cfg(test)]
mod fingerprint_tests {
    use super::choose_fingerprint;

    #[test]
    fn moss_example() {
        // Example from page 4 of the MOSS paper adapted for robust winnowing
        // (as well as removing identical back-to-back hashes)
        let hashes = vec![
            77, 74, 42, 17, 98, 50, 17, 98, 8, 88, 67, 39, 77, 74, 42, 17, 98,
        ];
        let w = 4;
        let fingerprint = choose_fingerprint(&hashes, w);
        assert_eq!(fingerprint.hashes, vec![17, 8, 39, 17]);
    }

    #[test]
    fn identical_hashes() {
        let hashes = vec![1, 1, 1, 1, 1];
        let w = 2;
        let fingerprint = choose_fingerprint(&hashes, w);
        assert_eq!(fingerprint.hashes, vec![1]);
    }
}
