use std::collections::HashMap;

mod fingerprint;
pub mod lexer;
mod token;

// Returns a list of matches represented as the indices in the input list
// of the first and second occurrences of a match.
pub fn detect_plagiarism(documents: &[&str]) -> Vec<(usize, usize)> {
    const K: usize = 5;
    const T: usize = 9;

    // Maps a hash to the index of the document in which it was first seen
    // let mut hashes_seen: HashMap<u64, usize, IdentityHasherBuilder> = HashMap::with_hasher(IdentityHasherBuilder);
    let mut hashes_seen: HashMap<u64, usize> = HashMap::new();

    let mut matches: Vec<(usize, usize)> = vec![];

    for (index, document) in documents.iter().enumerate() {
        let characters = document.chars().collect();
        let fingerprint = fingerprint::fingerprint(K, T, characters);
        for hash in fingerprint.hashes {
            match hashes_seen.get(&hash) {
                Some(&first_index) if first_index == index => {}
                Some(&first_index) => {
                    matches.push((first_index, index));
                }
                None => {
                    hashes_seen.insert(hash, index);
                }
            }
        }
    }

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
        let matches = detect_plagiarism(&chapters);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let strings = vec!["aaaaaaaaa bbbbbbbbb", "bbbbbbbbb aaaaaaaaa", "aaaa c bbbb"];
        let matches = detect_plagiarism(&strings);

        assert!(matches.contains(&(0, 1)));
        assert!(!matches.contains(&(0, 2)));
        assert!(!matches.contains(&(1, 2)));
    }
}
