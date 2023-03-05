use std::collections::{HashMap, HashSet};

mod fingerprint;
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
    // TODO: Could use the hashes directly instead of re-hashing them.
    let mut hashes_seen: HashMap<u64, usize> = HashMap::new();

    // Keep matches in a hash set so that matches are not reported multiple times.
    let mut matches: HashSet<(usize, usize)> = HashSet::new();

    for (index, document) in documents.iter().enumerate() {
        // TODO: Figure out why using the string bytes directly doesn't work. (Would reduce runtime by ~30% for the Moby Dick test).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moby_dick() {
        let moby_dick = include_str!("moby_dick.txt");

        // Split Moby Dick into its chapters
        let chapters = moby_dick.split("CHAPTER").collect::<Vec<_>>();
        let matches = detect_plagiarism(25, 50, &chapters);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let strings = vec!["aaaaaaaaa bbbbbbbbb", "bbbbbbbbb aaaaaaaaa", "aaaa c bbbb"];
        let matches = detect_plagiarism(5, 9, &strings);

        assert_eq!(matches, vec![(0, 1)]);
    }
}
