use std::{
    collections::HashMap,
    hash::{BuildHasher, Hasher},
};

mod fingerprint;
pub mod lexer;
mod token;

struct IdentityHasher {
    hash: u64,
}

impl IdentityHasher {
    fn new() -> Self {
        Self { hash: 0 }
    }
}

impl Hasher for IdentityHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash <<= 8;
            self.hash |= *byte as u64;
        }
    }

    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }
}

struct IdentityHasherBuilder;

impl BuildHasher for IdentityHasherBuilder {
    type Hasher = IdentityHasher;

    fn build_hasher(&self) -> Self::Hasher {
        IdentityHasher::new()
    }
}

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
        let fingerprint = fingerprint::fingerprint(K, T, document);
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
        let sentences = vec![
            "aaaaaaaaaa bbbbbbbbbb ccccc dddd",
            "bbbbbbbbbb asdfjhaksjhdf",
            "aslkdafhskjfhd aaaaaaaaaa",
            "asdfjkhaskdjhf cccc",
            "dddd askldfjhaskjfdh",
        ];

        let matches = detect_plagiarism(&sentences);
        assert_eq!(matches, vec![(0, 1), (0, 2)]);
    }
}
