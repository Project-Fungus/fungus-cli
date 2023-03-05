use criterion::{black_box, criterion_group, criterion_main, Criterion};
use manual_analyzer::fingerprint;
use manual_analyzer::identity_hash::IdentityHashMap;
use rustc_hash::FxHashSet as HashSet;

fn detect_plagiarism_bytes(
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
        let characters = document.bytes().collect();

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

fn detect_plagiarism_chars(
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
        let characters = document.chars().collect();

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

fn bench_bytes_or_chars(c: &mut Criterion) {
    let mut group =
        c.benchmark_group("Detecting plagiarism within chapters of Moby Dick (bytes vs chars)");

    let moby_dick = include_str!("moby_dick.txt");

    // Split Moby Dick into its chapters
    let chapters = moby_dick.split("CHAPTER").collect::<Vec<_>>();

    group.throughput(criterion::Throughput::Bytes(moby_dick.len() as u64));

    group.bench_with_input("Bytes", &chapters, |b, chapters| {
        b.iter(|| detect_plagiarism_bytes(25, 50, black_box(chapters)))
    });
    group.bench_with_input("Chars", &chapters, |b, chapters| {
        b.iter(|| detect_plagiarism_chars(25, 50, black_box(chapters)))
    });
}

criterion_group!(benches, bench_bytes_or_chars);
criterion_main!(benches);
