use std::collections::HashMap;
use std::path::PathBuf;

use clap::ValueEnum;
use fingerprint::Fingerprint;
use identity_hash::IdentityHashMap;
use itertools::Itertools;
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
    path: PathBuf,
    #[serde(skip_serializing)]
    contents: &'a str,
}

impl<'a> File<'a> {
    pub fn new(project_name: &'a str, name: PathBuf, contents: &'a str) -> File<'a> {
        File {
            project_name,
            path: name,
            contents,
        }
    }
}

/// Contains information about the similarity of two projects.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProjectPair<'a> {
    /// Name of the first project.
    project1: &'a str,
    /// Name of the second project.
    project2: &'a str,
    /// Number of matches detected between the two projects.
    ///
    /// This counts distinct hashes that match between the two projects (e.g., if project 1 contains the hash twice and project 3 has the same hash three times, that is just one match).
    num_matches: usize,
    /// Matches between the two projects.
    matches: Vec<Match>,
}

/// Contains information about a specific code snippet that is shared between two projects.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Match {
    /// List of places in which the code snippet appears in project 1.
    project1_occurrences: Vec<Location>,
    /// List of places in which the code snipet appears in project 2.
    project2_occurrences: Vec<Location>,
}

/// Absolute reference to a code snippet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Location {
    /// File in which the code snippet is found.
    file: PathBuf,
    /// Position of the code snippet within the file (in bytes).
    span: Span,
}

/// Detects matches between files in different projects and constructs a summary of the results.
///
/// Matches of length less than `noise_threshold` are guaranteed to be ignored.
/// Matches of length at least `guarantee_threshold` are guaranteed to be included.
pub fn detect_plagiarism<'a>(
    noise_threshold: usize,
    guarantee_threshold: usize,
    tokenizing_strategy: TokenizingStrategy,
    documents: &'a [&File<'a>],
    min_matches: usize,
) -> Vec<ProjectPair<'a>> {
    let document_fingerprints = documents.iter().map(|&d| {
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

    let hash_locations = build_hash_database(document_fingerprints);

    let mut project_pairs: HashMap<(&str, &str), Vec<Match>> = HashMap::default();
    for (_, locations) in hash_locations.iter() {
        let matches = locations_to_matches(locations);

        for (project1, project2, m) in matches {
            match project_pairs.get_mut(&(project1, project2)) {
                None => {
                    project_pairs.insert((project1, project2), vec![m]);
                }
                Some(lst) => {
                    lst.push(m);
                }
            }
        }
    }

    project_pairs
        .iter()
        .map(|((p1, p2), matches)| ProjectPair {
            project1: p1,
            project2: p2,
            num_matches: matches.len(),
            matches: matches.to_owned(),
        })
        .filter(|p| p.num_matches >= min_matches)
        .sorted_unstable_by(|x, y| y.num_matches.cmp(&x.num_matches))
        .collect()
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
            let characters = characters
                .iter()
                .enumerate()
                .map(|(i, &c)| (c, i..i + 1))
                .collect::<Vec<_>>();
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

fn build_hash_database<'a, I>(fingerprints: I) -> IdentityHashMap<Vec<(&'a File<'a>, Span)>>
where
    I: IntoIterator<Item = (&'a File<'a>, Fingerprint)>,
{
    let mut hash_locations: IdentityHashMap<Vec<(&File, Span)>> = IdentityHashMap::default();

    for (doc, fingerprint) in fingerprints.into_iter() {
        for (hash, span) in fingerprint.spanned_hashes {
            match hash_locations.get_mut(&hash) {
                None => {
                    hash_locations.insert(hash, vec![(doc, span)]);
                }
                Some(lst) => {
                    lst.push((doc, span));
                }
            }
        }
    }

    hash_locations
}

fn locations_to_matches<'a>(locations: &[(&'a File<'a>, Span)]) -> Vec<(&'a str, &'a str, Match)> {
    let grouped_locations = group_locations(locations);

    let mut matches = Vec::new();
    for (project1, project1_occurrences) in grouped_locations.to_owned() {
        for (project2, project2_occurrences) in grouped_locations.to_owned() {
            if project1 >= project2 {
                continue;
            }

            let m = Match {
                project1_occurrences: project1_occurrences.to_owned(),
                project2_occurrences: project2_occurrences.to_owned(),
            };
            matches.push((project1, project2, m));
        }
    }

    matches
}

fn group_locations<'a>(locations: &[(&'a File<'a>, Span)]) -> HashMap<&'a str, Vec<Location>> {
    let mut grouped_locations: HashMap<&str, Vec<Location>> = HashMap::default();

    for (file, span) in locations {
        let location = Location {
            file: file.path.to_owned(),
            span: span.to_owned(),
        };
        match grouped_locations.get_mut(file.project_name) {
            None => {
                grouped_locations.insert(file.project_name, vec![location]);
            }
            Some(lst) => {
                lst.push(location);
            }
        }
    }

    grouped_locations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moby_dick() {
        let moby_dick = include_str!("../benches/moby_dick.txt");

        // Split Moby Dick into its chapters
        let chapters = moby_dick
            .split("CHAPTER")
            .enumerate()
            .map(|(i, x)| (format!("Moby Dick Chapter {i}"), x))
            .collect::<Vec<_>>();
        let documents = chapters
            .iter()
            .map(|(project_name, contents)| File::new(project_name, "Chapter".into(), contents))
            .collect::<Vec<_>>();
        let document_references = documents.iter().collect::<Vec<_>>();
        let matches = detect_plagiarism(25, 50, TokenizingStrategy::Bytes, &document_references, 0);
        println!("{} matches found!", matches.len());
    }

    #[test]
    fn simple_sentences() {
        let file1 = File::new("P1", "C:/P1/file.txt".into(), "aaabbbzyxaaa");
        let file2 = File::new("P2", "C:/P2/file.txt".into(), "bbbaaa");
        let file3 = File::new("P3", "C:/P3/file.txt".into(), "acb");

        let documents = vec![&file1, &file2, &file3];
        let matches = detect_plagiarism(3, 3, TokenizingStrategy::Bytes, &documents, 0);

        assert_eq!(
            matches,
            vec![ProjectPair {
                project1: "P1",
                project2: "P2",
                num_matches: 2,
                matches: vec![
                    // The matches and locations are in no particular order?
                    // TODO: Specify the order in which they should be?
                    Match {
                        project1_occurrences: vec![Location {
                            file: "C:/P1/file.txt".into(),
                            span: 3..6
                        }],
                        project2_occurrences: vec![Location {
                            file: "C:/P2/file.txt".into(),
                            span: 0..3,
                        }],
                    },
                    Match {
                        project1_occurrences: vec![
                            Location {
                                file: "C:/P1/file.txt".into(),
                                span: 0..3
                            },
                            Location {
                                file: "C:/P1/file.txt".into(),
                                span: 9..12
                            }
                        ],
                        project2_occurrences: vec![Location {
                            file: "C:/P2/file.txt".into(),
                            span: 3..6
                        }]
                    },
                ]
            }]
        );
    }
}
