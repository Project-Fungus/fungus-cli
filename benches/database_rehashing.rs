use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use manual_analyzer::fingerprint;
use manual_analyzer::identity_hash::IdentityHashMap;
use rustc_hash::{FxHashMap, FxHashSet as HashSet};

fn detect_plagiarism_rehashing_default(
    noise_threshold: usize,
    guarantee_threshold: usize,
    documents: &[&str],
) -> Vec<(usize, usize)> {
    // Maps a hash to the index of the document in which it was first seen
    let mut hashes_seen: HashMap<u64, usize> = HashMap::default();

    // Keep matches in a hash set so that matches are not reported multiple times.
    let mut matches: HashSet<(usize, usize)> = HashSet::default();

    for (index, document) in documents.iter().enumerate() {
        // Use bytes instead of chars since it shouldn't affect the result and is faster.
        let characters = document.as_bytes();

        let fingerprint =
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, characters);

        for hash in fingerprint.hashes {
            match hashes_seen.get(&hash) {
                Some(&first_index) if first_index == index => {}
                Some(&first_index) => {
                    matches.insert((first_index, index));
                }
                None => {
                    hashes_seen.insert(hash, index);
                }
            }
        }
    }

    let mut matches: Vec<_> = matches.into_iter().collect();
    matches.sort();

    matches
}

fn detect_plagiarism_rehashing_rustc_hash(
    noise_threshold: usize,
    guarantee_threshold: usize,
    documents: &[&str],
) -> Vec<(usize, usize)> {
    // Maps a hash to the index of the document in which it was first seen
    let mut hashes_seen: FxHashMap<u64, usize> = FxHashMap::default();

    // Keep matches in a hash set so that matches are not reported multiple times.
    let mut matches: HashSet<(usize, usize)> = HashSet::default();

    for (index, document) in documents.iter().enumerate() {
        // Use bytes instead of chars since it shouldn't affect the result and is faster.
        let characters = document.as_bytes();

        let fingerprint =
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, characters);

        for hash in fingerprint.hashes {
            match hashes_seen.get(&hash) {
                Some(&first_index) if first_index == index => {}
                Some(&first_index) => {
                    matches.insert((first_index, index));
                }
                None => {
                    hashes_seen.insert(hash, index);
                }
            }
        }
    }

    let mut matches: Vec<_> = matches.into_iter().collect();
    matches.sort();

    matches
}

fn detect_plagiarism_no_rehashing(
    noise_threshold: usize,
    guarantee_threshold: usize,
    documents: &[&str],
) -> Vec<(usize, usize)> {
    // Maps a hash to the index of the document in which it was first seen
    let mut hashes_seen: IdentityHashMap<usize> = IdentityHashMap::default();

    // Keep matches in a hash set so that matches are not reported multiple times.
    let mut matches: HashSet<(usize, usize)> = HashSet::default();

    for (index, document) in documents.iter().enumerate() {
        // Use bytes instead of chars since it shouldn't affect the result and is faster.
        let characters = document.as_bytes();

        let fingerprint =
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, characters);

        for hash in fingerprint.hashes {
            match hashes_seen.get(&hash) {
                Some(&first_index) if first_index == index => {}
                Some(&first_index) => {
                    matches.insert((first_index, index));
                }
                None => {
                    hashes_seen.insert(hash, index);
                }
            }
        }
    }

    let mut matches: Vec<_> = matches.into_iter().collect();
    matches.sort();

    matches
}

fn bench_database_rehashing(c: &mut Criterion) {
    let mut group =
        c.benchmark_group("Detecting plagiarism within chapters of Moby Dick (database rehashing)");

    let moby_dick = include_str!("moby_dick.txt");

    // Split Moby Dick into its chapters
    let chapters = moby_dick.split("CHAPTER").collect::<Vec<_>>();

    group.throughput(criterion::Throughput::Bytes(moby_dick.len() as u64));

    group.bench_with_input("Rehashing (default hasher)", &chapters, |b, chapters| {
        b.iter(|| detect_plagiarism_rehashing_default(25, 50, black_box(chapters)))
    });
    group.bench_with_input("Rehashing (rustc hasher)", &chapters, |b, chapters| {
        b.iter(|| detect_plagiarism_rehashing_rustc_hash(25, 50, black_box(chapters)))
    });
    group.bench_with_input("No rehashing", &chapters, |b, chapters| {
        b.iter(|| detect_plagiarism_no_rehashing(25, 50, black_box(chapters)))
    });
}

criterion_group!(benches, bench_database_rehashing);
criterion_main!(benches);
