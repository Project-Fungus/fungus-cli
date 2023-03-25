use clap::ValueEnum;
use identity_hash::IdentityHashMap;
use lexing::naive::lex;
use lexing::relative::lex as lex_relative;
use rustc_hash::FxHashSet as HashSet;

pub mod fingerprint;
pub mod identity_hash;
pub mod lexing;

#[derive(Debug, Clone, ValueEnum)]
pub enum TokenizingStrategy {
    /// Do not tokenize the input. Instead, process the input as a sequence of bytes.
    Bytes,
    /// Tokenize the input using a best-effort, naive GNU ARMv7 assembly tokenizer.
    Naive,
    /// Tokenize the input using a more conservative and transformation-resistant GNU ARM assembly tokenizer.
    ///
    /// This tokenizer represents symbols using relative offsets from their last occurrence in the token sequence.
    /// This requires an additional pass over the input to compute the offsets and identify key symbols
    /// (i.e. instructions and directives).
    Relative,
}

/// Returns a list of matches represented as the indices in the input list
/// of the first and second occurrences of a match.
///
/// Matches of length less than `noise_threshold` are guaranteed to be ignored.
/// Matches of length at least `guarantee_threshold` are guaranteed to be included.
pub fn detect_plagiarism<S: AsRef<str>>(
    noise_threshold: usize,
    guarantee_threshold: usize,
    tokenizing_strategy: TokenizingStrategy,
    documents: &[S],
) -> Vec<(usize, usize)> {
    // Maps a hash to the index of the document in which it was first seen
    // To prevent rehashing the hashes, we use a hash map which does not rehash the keys.
    let mut hashes_seen: IdentityHashMap<usize> = IdentityHashMap::default();

    // Keep matches in a hash set so that matches are not reported multiple times.
    let mut matches: HashSet<(usize, usize)> = HashSet::default();

    for (index, document) in documents.iter().enumerate() {
        let fingerprint = match tokenizing_strategy {
            TokenizingStrategy::Bytes => {
                // Use bytes instead of chars since it shouldn't affect the result and is faster.
                let characters = document.as_ref().as_bytes();
                fingerprint::fingerprint(noise_threshold, guarantee_threshold, characters)
            }
            TokenizingStrategy::Naive => {
                let tokens = lex(document.as_ref());
                fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens)
            }
            TokenizingStrategy::Relative => {
                let tokens = lex_relative(document.as_ref());
                fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens)
            }
        };

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
    matches.sort_unstable();

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
        let matches = detect_plagiarism(25, 50, TokenizingStrategy::Bytes, &chapters);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let strings = vec!["aaabbb", "bbbaaa", "acb"];
        let matches = detect_plagiarism(2, 3, TokenizingStrategy::Bytes, &strings);

        assert_eq!(matches, vec![(0, 1)]);
    }
}
