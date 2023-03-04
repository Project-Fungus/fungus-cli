pub struct Fingerprint {
    pub hashes: Vec<u64>,
}

/// Generates a `Fingerprint` for the given `String` using the winnowing algorithm.
///
/// Substrings with length at least `t` are guaranteed to be captured in the fingerprint.
/// Substrings with length less than `k` are excluded from the fingerprint.
pub fn fingerprint(k: usize, t: usize, s: &str) -> Fingerprint {
    assert!(k <= t);
    assert!(k != 0);

    // The window size is set to t - k + 1 such that at least one hash is picked from every
    // sequence of hash of length greater than t - k.
    let w = t - k + 1;

    // Operate over the bytes of the string for performance.
    let bytes = s.bytes().collect::<Vec<_>>();

    // Generate the hashes of all valid k-grams in the document.
    // By hashing k-grams, we guarantee that no match shorter than k will be included in the
    // fingerprint.
    let hashes = hashes(k, &bytes);

    let mut fingerprint_hashes = vec![];
    let mut previously_picked_hash = None;

    for window in hashes.windows(w) {
        let &min_hash = window.iter().min().unwrap();
        match previously_picked_hash {
            Some(h) if h == min_hash => {}
            _ => {
                fingerprint_hashes.push(min_hash);
                previously_picked_hash = Some(min_hash);
            }
        };
    }

    Fingerprint {
        hashes: fingerprint_hashes,
    }
}

/// Generates a 64-bit hash for all windows of bytes of length `k` in `bytes` using a rolling
/// hash function.
fn hashes(k: usize, bytes: &[u8]) -> Vec<u64> {
    assert!(bytes.len() >= k);
    assert!(k > 0);
    assert!(u32::try_from(k).is_ok());

    // B is a prime number greater than the maximum value for a byte
    const B: u64 = 257;
    let mut hashes = Vec::with_capacity(bytes.len() - k + 1);
    let mut first_hash: u64 = 0;

    for (i, &byte) in bytes[0..k].iter().enumerate() {
        // acc + byte * B^(k - i)
        first_hash =
            first_hash.wrapping_add((u64::from(byte)).wrapping_mul(B.wrapping_pow((k - i) as u32)));
    }

    hashes.push(first_hash);

    let mut last_hash = first_hash;
    let mut next_byte_to_remove = bytes[0];

    for i in k..bytes.len() {
        let new_byte = bytes[i];
        let new_hash = last_hash
            .wrapping_sub(u64::from(next_byte_to_remove).wrapping_pow(k as u32))
            .wrapping_add(u64::from(new_byte))
            .wrapping_mul(k as u64);

        last_hash = new_hash;
        next_byte_to_remove = bytes[i - k + 1];
        hashes.push(new_hash);
    }

    hashes
}
