use clap::ValueEnum;
use fingerprint::Fingerprint;
use identity_hash::IdentityHashMap;
use lexing::naive::lex;
use lexing::relative::lex as lex_relative;
use logos::Span;
use serde::Serialize;

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

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct File<'a> {
    project_name: &'a str,
    name: &'a str,
    #[serde(skip_serializing)]
    contents: &'a str,
}

impl<'a> File<'a> {
    pub fn new(project_name: &'a str, name: &'a str, contents: &'a str) -> File<'a> {
        File {
            project_name,
            name,
            contents,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Match<'a> {
    file1: &'a File<'a>,
    file1_spans: Vec<Span>,
    file2: &'a File<'a>,
    file2_spans: Vec<Span>,
}

impl<'a> Match<'a> {
    pub fn new(
        file1: &'a File<'a>,
        file1_spans: Vec<Span>,
        file2: &'a File<'a>,
        file2_spans: Vec<Span>,
    ) -> Match<'a> {
        Match {
            file1,
            file1_spans,
            file2,
            file2_spans,
        }
    }
}

/// Returns a list of matches.
///
/// Matches of length less than `noise_threshold` are guaranteed to be ignored.
/// Matches of length at least `guarantee_threshold` are guaranteed to be included.
pub fn detect_plagiarism<'a>(
    noise_threshold: usize,
    guarantee_threshold: usize,
    tokenizing_strategy: TokenizingStrategy,
    documents: &'a [&File<'a>],
) -> Vec<Match<'a>> {
    let document_fingerprints = documents.iter().map(|d| {
        (
            d,
            fingerprint(
                d,
                &tokenizing_strategy,
                noise_threshold,
                guarantee_threshold,
            ),
        )
    });

    let mut matches = Vec::new();
    let mut hash_locations: IdentityHashMap<Vec<(&File, Vec<Span>)>> = IdentityHashMap::default();

    for (document, fingerprint) in document_fingerprints {
        let hash_locations_for_file = expand_fingerprint(document, fingerprint);
        for (&hash, (file, spans)) in hash_locations_for_file.iter() {
            match hash_locations.get_mut(&hash) {
                None => {
                    hash_locations.insert(hash, vec![(file, spans.clone())]);
                }
                Some(locations) if locations.is_empty() => {
                    locations.push((file, spans.clone()));
                }
                Some(locations) => {
                    for (other_file, other_spans) in locations {
                        let m = Match::new(other_file, other_spans.clone(), file, spans.clone());
                        matches.push(m);
                    }
                }
            }
        }
    }

    matches
}

fn fingerprint(
    document: &File,
    tokenizing_strategy: &TokenizingStrategy,
    noise_threshold: usize,
    guarantee_threshold: usize,
) -> Fingerprint {
    match tokenizing_strategy {
        TokenizingStrategy::Bytes => {
            // Use bytes instead of chars since it shouldn't affect the result and is faster.
            let characters = document.contents.as_bytes();
            // TODO: More efficient way of doing this?
            let characters = characters.iter().map(|&c| (c, 0..0)).collect::<Vec<_>>();
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, &characters)
        }
        TokenizingStrategy::Naive => {
            let tokens = lex(document.contents);
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens)
        }
        TokenizingStrategy::Relative => {
            let tokens = lex_relative(document.contents);
            // TODO: Update relative lexer as well
            let tokens = tokens.iter().map(|t| (t, 0..1)).collect::<Vec<_>>();
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens)
        }
    }
}

fn expand_fingerprint<'a>(
    document: &'a File<'a>,
    fingerprint: Fingerprint,
) -> IdentityHashMap<(&'a File<'a>, Vec<Span>)> {
    let mut hash_locations = IdentityHashMap::default();

    for (hash, span) in fingerprint.spanned_hashes {
        match hash_locations.get_mut(&hash) {
            None => {
                hash_locations.insert(hash, (document, vec![span]));
            }
            Some((_, spans)) => {
                spans.push(span);
            }
        }
    }

    hash_locations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moby_dick() {
        let moby_dick = include_str!("../benches/moby_dick.txt");

        // Split Moby Dick into its chapters
        let chapters = moby_dick.split("CHAPTER").collect::<Vec<_>>();
        let documents = chapters
            .iter()
            .enumerate()
            .map(|(_, &s)| File::new("Moby Dick", "Chapter", s))
            .collect::<Vec<_>>();
        let document_references = documents.iter().collect::<Vec<_>>();
        let matches = detect_plagiarism(25, 50, TokenizingStrategy::Bytes, &document_references);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let file1 = File::new("String 1", "String 1", "aaabbb");
        let file2 = File::new("String 2", "String 2", "bbbaaa");
        let file3 = File::new("String 3", "String 3", "acb");

        let documents = vec![&file1, &file2, &file3];
        let matches = detect_plagiarism(2, 3, TokenizingStrategy::Bytes, &documents);

        assert_eq!(
            matches,
            vec![
                Match::new(&file1, vec![0..3], &file2, vec![3..6]),
                Match::new(&file1, vec![3..6], &file2, vec![0..3])
            ]
        );
    }
}
