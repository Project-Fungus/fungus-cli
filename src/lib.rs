use std::hash::Hash;

use identity_hash::IdentityHashMap;
use lexer::{lex, Token};
use rustc_hash::FxHashSet as HashSet;

pub mod fingerprint;
pub mod identity_hash;
pub mod lexer;

/// Returns a list of matches represented as the indices in the input list
/// of the first and second occurrences of a match.
///
/// Matches of length less than `noise_threshold` are guaranteed to be ignored.
/// Matches of length at least `guarantee_threshold` are guaranteed to be included.
pub fn detect_plagiarism<S: AsRef<str>>(
    noise_threshold: usize,
    guarantee_threshold: usize,
    should_lex: bool,
    documents: &[S],
) -> Vec<(usize, usize)> {
    if should_lex {
        let lexed_documents = documents
            .iter()
            .map(|document| lex(document.as_ref()))
            .collect::<Vec<_>>();

        // First pass does no transformation on the original documents
        let lexed_documents_no_transformation = lexed_documents.iter();

        // Second pass ignores whitespace and comments
        let lexed_documents_no_whitespace_no_comments = lexed_documents.iter().map(|document| {
            document
                .iter()
                .filter(|&token| token != &Token::Whitespace && matches!(token, Token::Comment(_)))
        });

        // Third pass additionally ignores register names
        let lexed_documents_no_register_names = lexed_documents_no_whitespace_no_comments
            .clone()
            .map(|document| {
                document.map(|token| match token {
                    Token::Register(_) => &Token::Register(0),
                    _ => token,
                })
            });

        // Return matches from all passes
        // TODO: Change noise and guarantee thresholds to be per-pass
        [
            detect_plagiarism_generic(
                noise_threshold,
                guarantee_threshold,
                lexed_documents_no_transformation,
            ),
            detect_plagiarism_generic(
                noise_threshold,
                guarantee_threshold,
                lexed_documents_no_whitespace_no_comments,
            ),
            detect_plagiarism_generic(
                noise_threshold,
                guarantee_threshold,
                lexed_documents_no_register_names,
            ),
        ]
        .concat()
    } else {
        let document_bytes = documents
            .iter()
            .map(|document| document.as_ref().as_bytes());

        detect_plagiarism_generic(noise_threshold, guarantee_threshold, document_bytes)
    }
}

fn detect_plagiarism_generic<Docs, Doc, Token>(
    noise_threshold: usize,
    guarantee_threshold: usize,
    documents: Docs,
) -> Vec<(usize, usize)>
where
    Docs: IntoIterator<Item = Doc>,
    Doc: IntoIterator<Item = Token>,
    Token: Hash,
{
    // Maps a hash to the index of the document in which it was first seen
    // To prevent rehashing the hashes, we use a hash map which does not rehash the keys.
    let mut hashes_seen: IdentityHashMap<usize> = IdentityHashMap::default();

    // Keep matches in a hash set so that matches are not reported multiple times.
    let mut matches: HashSet<(usize, usize)> = HashSet::default();

    for (index, document) in documents.into_iter().enumerate() {
        let tokens = document.into_iter().collect::<Vec<Token>>();
        let fingerprint = fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens);

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
        let matches = detect_plagiarism(25, 50, false, &chapters);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let strings = vec!["aaabbb", "bbbaaa", "acb"];
        let matches = detect_plagiarism(2, 3, false, &strings);

        assert_eq!(matches, vec![(0, 1)]);
    }
}
