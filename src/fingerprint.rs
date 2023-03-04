use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub struct Fingerprint {
    pub hashes: Vec<u64>,
}

/// Generates a `Fingerprint` for the given vector of tokens using the winnowing algorithm.
/// Tokens can be any type that implements the `Hash` trait (chars, tokens from a lexer, etc.).
///
/// Substrings with length at least `t` are guaranteed to be captured in the fingerprint.
/// Substrings with length less than `k` are excluded from the fingerprint.
pub fn fingerprint<T>(k: usize, t: usize, tokens: Vec<T>) -> Fingerprint
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
    let hashes = hash_tokens(tokens, k);

    choose_fingerprint(hashes, w)
}

fn hash_tokens<T>(tokens: Vec<T>, k: usize) -> Vec<u64>
where
    T: Hash,
{
    tokens.windows(k).map(|w| hash_window(w)).collect()
}

fn hash_window<T>(tokens: &[T]) -> u64
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();
    tokens.hash(&mut hasher);
    hasher.finish()
}

fn choose_fingerprint(hashes: Vec<u64>, w: usize) -> Fingerprint {
    let mut fingerprint_hashes = vec![];
    let mut previously_picked_hash = None;

    for window_start in 0..=(hashes.len() - w) {
        let max_idx_in_window = window_start + w - 1;
        let mut min_hash = (hashes[max_idx_in_window], max_idx_in_window);
        for i in (window_start..=max_idx_in_window).rev() {
            if hashes[i] < min_hash.0 {
                min_hash = (hashes[i], i);
            }
        }

        match previously_picked_hash {
            // If the previously-picked hash is still in this window and the new
            // value is not lower, then just keep the old value
            Some((v, i)) if i >= window_start && v <= min_hash.0 => {}
            _ => {
                previously_picked_hash = Some(min_hash);
                fingerprint_hashes.push(min_hash.0);
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
        // Example from page 4 of the MOSS paper
        let hashes = vec![
            77, 74, 42, 17, 98, 50, 17, 98, 8, 88, 67, 39, 77, 74, 42, 17, 98,
        ];
        let w = 4;
        let fingerprint = choose_fingerprint(hashes, w);
        assert_eq!(fingerprint.hashes, vec![17, 17, 8, 39, 17]);
    }

    #[test]
    fn identical_hashes() {
        let hashes = vec![1, 1, 1, 1, 1];
        let w = 2;
        let fingerprint = choose_fingerprint(hashes, w);
        assert_eq!(fingerprint.hashes, vec![1, 1]);
    }
}
