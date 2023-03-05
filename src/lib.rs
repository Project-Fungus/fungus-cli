use identity_hash::IdentityHashMap;
use rustc_hash::FxHashSet as HashSet;

pub mod fingerprint;
pub mod identity_hash;
pub mod lexer;
mod token;

/// Returns a list of matches represented as the indices in the input list
/// of the first and second occurrences of a match.
///
/// Matches of length less than `noise_threshold` are guaranteed to be ignored.
/// Matches of length at least `guarantee_threshold` are guaranteed to be included.
pub fn detect_plagiarism(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moby_dick() {
        let moby_dick = include_str!("../benches/moby_dick.txt");

        // Split Moby Dick into its chapters
        let chapters = moby_dick.split("CHAPTER").collect::<Vec<_>>();
        let matches = detect_plagiarism(25, 50, &chapters);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let strings = vec!["aaabbb", "bbbaaa", "acb"];
        let matches = detect_plagiarism(2, 3, &strings);

        assert_eq!(matches, vec![(0, 1)]);
    }
}
