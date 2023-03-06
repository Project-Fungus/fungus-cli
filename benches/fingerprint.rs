use criterion::{black_box, criterion_group, criterion_main, Criterion};
use manual_analyzer::fingerprint::Fingerprint;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

#[inline]
fn hash_window<T>(tokens: &[T]) -> u64
where
    T: Hash,
{
    let mut hasher = FxHasher::default();
    tokens.hash(&mut hasher);
    hasher.finish()
}

pub fn fingerprint_original<T>(k: usize, t: usize, tokens: &[T]) -> Fingerprint
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

    choose_fingerprint_original(&hashes, w)
}

fn choose_fingerprint_original(hashes: &[u64], w: usize) -> Fingerprint {
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

pub fn fingerprint_new<T>(k: usize, t: usize, tokens: &[T]) -> Fingerprint
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

    choose_fingerprint_new(&hashes, w)
}

#[inline]
fn min_with_index(hashes: &[u64]) -> (u64, usize) {
    let mut min = hashes[0];
    let mut min_index = 0;

    for (i, &hash) in hashes.iter().enumerate() {
        if hash < min {
            min = hash;
            min_index = i;
        }
    }

    (min, min_index)
}

fn choose_fingerprint_new(hashes: &[u64], w: usize) -> Fingerprint {
    // Guarantee that the length is at least one window.
    if hashes.len() < w {
        return Fingerprint { hashes: vec![] };
    }

    // First min hash.
    let first_window = &hashes[0..w];
    let (mut min_hash, mut min_index) = min_with_index(first_window);

    let mut fingerprint_hashes = vec![min_hash];

    for i in 0..hashes.len() - w {
        // let window = &hashes[i..i + w];
        if min_index < i {
            // The min hash is no longer in the window. Find a new min hash.
            let new_window = &hashes[i..i + w];
            let previous_min_hash = min_hash;
            (min_hash, min_index) = min_with_index(new_window);

            if min_hash != previous_min_hash {
                fingerprint_hashes.push(min_hash);
            }
        } else {
            // The min hash is still in the window. Check if the next hash is smaller.
            let next_hash = hashes[i + w - 1];
            if next_hash < min_hash {
                min_hash = next_hash;
                min_index = i + w - 1;
                fingerprint_hashes.push(min_hash);
            }
        }
    }

    Fingerprint {
        hashes: fingerprint_hashes,
    }
}

fn bench_fingerprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fingerprinting Moby Dick");

    let moby_dick = include_str!("moby_dick.txt");
    let moby_dick_bytes = moby_dick.as_bytes();

    group.throughput(criterion::Throughput::Bytes(moby_dick.len() as u64));

    group.bench_with_input("Original", &moby_dick_bytes, |b, &moby_dick_bytes| {
        b.iter(|| fingerprint_original(25, 50, black_box(moby_dick_bytes)))
    });
    group.bench_with_input("New", &moby_dick_bytes, |b, moby_dick_bytes| {
        b.iter(|| fingerprint_new(25, 50, black_box(moby_dick_bytes)))
    });
}

criterion_group!(benches, bench_fingerprint);
criterion_main!(benches);
