use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use clap::ValueEnum;
use fingerprint::Fingerprint;
use identity_hash::IdentityHashMap;
use itertools::Itertools;
use lexing::naive::lex;
use lexing::relative::lex as lex_relative;
use output::{Location, Match, ProjectPair, Warning, WarningType};
use preprocessing::whitespace_removal::remove_whitespace_relative;

mod fingerprint;
pub mod identity_hash;
mod lexing;
pub mod output;
mod preprocessing;

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct File {
    project: PathBuf,
    path: PathBuf,
    contents: String,
}

impl File {
    pub fn new(project_name: PathBuf, path: PathBuf, contents: String) -> File {
        File {
            project: project_name,
            path,
            contents,
        }
    }
}

/// Detects matches between files in different projects and constructs a summary of the results.
///
/// Matches of length less than `noise_threshold` are guaranteed to be ignored.
/// Matches of length at least `guarantee_threshold` are guaranteed to be included.
#[allow(clippy::too_many_arguments)]
pub fn detect_plagiarism(
    noise_threshold: usize,
    guarantee_threshold: usize,
    tokenizing_strategy: TokenizingStrategy,
    ignore_whitespace: bool,
    min_matches: usize,
    common_hash_threshold: Option<f64>,
    documents: &[File],
    ignored_documents: &[File],
) -> (Vec<ProjectPair>, Vec<Warning>) {
    let (project_fingerprints, mut warnings) = fingerprint_multiple(
        documents,
        &tokenizing_strategy,
        noise_threshold,
        guarantee_threshold,
        ignore_whitespace,
    );

    let (ignored_fingerprints, mut ignored_doc_warnings) = fingerprint_multiple(
        ignored_documents,
        &tokenizing_strategy,
        noise_threshold,
        // Use the same noise and guarantee threshold so that the window size is 1.
        //
        // Suppose the window size was 2. Suppose the hashes from the starter code were [0, 5] and the hashes from the
        // assignment code were [..., 0, 5, 6, ...]. In the starter code, the fingerprint would be {0}. In the
        // assignment code, the fingerprint would be {..., 0, 5, ...}. Only the hash 0 would be discarded, not 5 (even
        // though 5 matches starter code). If the window size is set to 1 for the starter code, any code snippet that
        // fully matches _any_ part of the starter code is guaranteed to be ignored.
        //
        // Letting the window size be 1 for starter code shouldn't have a huge impact on performance, since there's
        // normally less starter code than assignment code. Normally, starter code is a strict subset of each student's
        // submission and there are many students.
        noise_threshold,
        ignore_whitespace,
    );
    let ignored_hashes = ignored_fingerprints
        .iter()
        .flat_map(|(_, f)| &f.spanned_hashes)
        .map(|(hash, _)| *hash)
        .collect::<Vec<_>>();
    warnings.append(&mut ignored_doc_warnings);

    // Map hashes to their locations
    let mut hash_locations = build_hash_database(project_fingerprints);

    let num_projects = documents
        .iter()
        .map(|f| &f.project)
        .sorted()
        .dedup()
        .count();

    filter_hashes(
        &mut hash_locations,
        &ignored_hashes,
        num_projects,
        common_hash_threshold,
    );

    // Turn each set of locations that share a hash into a set of "matches" between distinct projects
    let mut project_pairs: HashMap<(&PathBuf, &PathBuf), Vec<Match>> = HashMap::default();
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

    let mut project_pairs = project_pairs
        .iter()
        .map(|((p1, p2), matches)| ProjectPair {
            project1: (*p1).to_owned(),
            project2: (*p2).to_owned(),
            num_matches: matches.len(),
            matches: matches.to_owned(),
        })
        .filter(|p| p.num_matches >= min_matches)
        .collect();
    sort_output(&mut project_pairs);

    (project_pairs, warnings)
}

fn fingerprint_multiple<'a>(
    documents: &'a [File],
    tokenizing_strategy: &TokenizingStrategy,
    noise_threshold: usize,
    guarantee_threshold: usize,
    ignore_whitespace: bool,
) -> (Vec<(&'a File, Fingerprint)>, Vec<Warning>) {
    let fingerprint_results = documents.iter().map(|d| {
        (
            d,
            fingerprint(
                d,
                tokenizing_strategy,
                noise_threshold,
                guarantee_threshold,
                ignore_whitespace,
            ),
        )
    });

    let mut fingerprints = Vec::new();
    let mut warnings = Vec::new();
    for (document, result) in fingerprint_results {
        match result {
            Err(e) => {
                warnings.push(Warning {
                    file: Some(document.path.to_owned()),
                    message: e.to_string(),
                    warn_type: WarningType::Fingerprint,
                });
            }
            Ok(f) => {
                fingerprints.push((document, f));
            }
        }
    }

    (fingerprints, warnings)
}

/// Produces the fingerprint for a single file using the given tokenization strategy.
fn fingerprint(
    document: &File,
    tokenizing_strategy: &TokenizingStrategy,
    noise_threshold: usize,
    guarantee_threshold: usize,
    ignore_whitespace: bool,
) -> anyhow::Result<Fingerprint> {
    match tokenizing_strategy {
        TokenizingStrategy::Bytes => {
            // Use bytes instead of chars since it shouldn't affect the result and is faster.
            let characters = document.contents.as_bytes();
            let characters = characters
                .iter()
                .enumerate()
                .map(|(i, &c)| (c, i..i + 1))
                .collect::<Vec<_>>();
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, &characters)
        }
        TokenizingStrategy::Naive => {
            let tokens = lex(&document.contents);
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens)
        }
        TokenizingStrategy::Relative => {
            let mut tokens = lex_relative(&document.contents);
            if ignore_whitespace {
                tokens = remove_whitespace_relative(tokens);
            }
            fingerprint::fingerprint(noise_threshold, guarantee_threshold, &tokens)
        }
    }
}

/// Constructs a "hash database" that maps a hash to all the locations in which it was found in the code.
fn build_hash_database<'a, I>(fingerprints: I) -> IdentityHashMap<Vec<(&'a File, Range<usize>)>>
where
    I: IntoIterator<Item = (&'a File, Fingerprint)>,
{
    let mut hash_locations: IdentityHashMap<Vec<(&File, Range<usize>)>> =
        IdentityHashMap::default();

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

fn filter_hashes(
    hash_database: &mut IdentityHashMap<Vec<(&File, Range<usize>)>>,
    ignored_hashes: &[u64],
    num_projects: usize,
    common_hash_threshold: Option<f64>,
) {
    for h in ignored_hashes {
        hash_database.remove(h);
    }

    if let Some(c) = common_hash_threshold {
        let mut hashes_to_discard = Vec::new();
        for (&hash, locations) in hash_database.iter() {
            let this_num_projects = locations
                .iter()
                .map(|(f, _)| &f.project)
                .sorted()
                .dedup()
                .count();
            if (this_num_projects as f64) >= (num_projects as f64) * c {
                hashes_to_discard.push(hash);
            }
        }

        for h in hashes_to_discard {
            hash_database.remove(&h);
        }
    }
}

/// Converts a set of locations (i.e., identical code snippets) into a set of matches between distinct projects.
fn locations_to_matches<'a>(
    locations: &[(&'a File, Range<usize>)],
) -> Vec<(&'a PathBuf, &'a PathBuf, Match)> {
    let grouped_locations = group_locations(locations);

    let mut matches = Vec::new();
    for (&project1, project1_occurrences) in grouped_locations.iter() {
        for (&project2, project2_occurrences) in grouped_locations.iter() {
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

/// Groups a set of locations by project.
fn group_locations<'a>(
    locations: &[(&'a File, Range<usize>)],
) -> HashMap<&'a PathBuf, Vec<Location>> {
    let mut grouped_locations: HashMap<&PathBuf, Vec<Location>> = HashMap::default();

    for (file, span) in locations {
        let location = Location {
            file: file.path.to_owned(),
            span: span.to_owned(),
        };
        match grouped_locations.get_mut(&file.project) {
            None => {
                grouped_locations.insert(&file.project, vec![location]);
            }
            Some(lst) => {
                lst.push(location);
            }
        }
    }

    grouped_locations
}

/// Sorts the project pairs, the matches, and the locations.
fn sort_output(project_pairs: &mut Vec<ProjectPair>) {
    project_pairs.sort_unstable_by_key(|p| p.num_matches);
    project_pairs.reverse();

    for pp in project_pairs {
        for m in pp.matches.iter_mut() {
            m.project1_occurrences.sort_unstable_by(|l1, l2| {
                (&l1.file, l1.span.start).cmp(&(&l2.file, l2.span.start))
            });
            m.project2_occurrences.sort_unstable_by(|l1, l2| {
                (&l1.file, l1.span.start).cmp(&(&l2.file, l2.span.start))
            });
        }

        pp.matches.sort_unstable_by(|m1, m2| {
            let m1_first_location = m1.project1_occurrences.first().unwrap();
            let m2_first_location = m2.project1_occurrences.first().unwrap();
            (&m1_first_location.file, m1_first_location.span.start)
                .cmp(&(&m2_first_location.file, m2_first_location.span.start))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_sentences() {
        let file3 = File::new("P1".into(), "C:/P1/file1.txt".into(), "aaa".to_owned());
        let file1 = File::new(
            "P1".into(),
            "C:/P1/file2.txt".into(),
            "aaabbbzyxaaa123ccc".to_owned(),
        );
        let file2 = File::new("P2".into(), "C:/P2/file.txt".into(), "bbbaaaccc".to_owned());
        let file4 = File::new("P3".into(), "C:/P3/file.txt".into(), "acb".to_owned());

        let documents = vec![file1, file2, file3, file4];
        let (matches, warnings) = detect_plagiarism(
            3,
            3,
            TokenizingStrategy::Bytes,
            false,
            0,
            None,
            &documents,
            &[],
        );

        assert!(warnings.is_empty());
        assert_eq!(
            matches,
            vec![ProjectPair {
                project1: "P1".into(),
                project2: "P2".into(),
                num_matches: 3,
                matches: vec![
                    Match {
                        project1_occurrences: vec![
                            Location {
                                file: "C:/P1/file1.txt".into(),
                                span: 0..3
                            },
                            Location {
                                file: "C:/P1/file2.txt".into(),
                                span: 0..3
                            },
                            Location {
                                file: "C:/P1/file2.txt".into(),
                                span: 9..12
                            }
                        ],
                        project2_occurrences: vec![Location {
                            file: "C:/P2/file.txt".into(),
                            span: 3..6
                        }]
                    },
                    Match {
                        project1_occurrences: vec![Location {
                            file: "C:/P1/file2.txt".into(),
                            span: 3..6
                        }],
                        project2_occurrences: vec![Location {
                            file: "C:/P2/file.txt".into(),
                            span: 0..3,
                        }],
                    },
                    Match {
                        project1_occurrences: vec![Location {
                            file: "C:/P1/file2.txt".into(),
                            span: 15..18,
                        }],
                        project2_occurrences: vec![Location {
                            file: "C:/P2/file.txt".into(),
                            span: 6..9
                        }],
                    }
                ]
            }]
        );
    }

    #[test]
    fn small_files() {
        let file = File::new("Project".into(), "File".into(), "Hello there!".to_owned());
        let ignored_file = File::new(
            "Ignored Project".into(),
            "Ignored File".into(),
            "Contents".to_owned(),
        );
        let noise = 1000;
        let guarantee = 1500;

        let (project_pairs, warnings) = detect_plagiarism(
            noise,
            guarantee,
            TokenizingStrategy::Bytes,
            false,
            5,
            None,
            &[file.to_owned()],
            &[ignored_file.to_owned()],
        );

        assert!(project_pairs.is_empty());
        assert_eq!(
            warnings,
            vec![
                Warning {
                    file: Some("File".into()),
                    message: format!("File could not be fingerprinted because it contains {} tokens, which is less than the noise threshold of {}.", &file.contents.len(), noise),
                    warn_type: WarningType::Fingerprint,
                },
                Warning {
                    file: Some("Ignored File".into()),
                    message: format!("File could not be fingerprinted because it contains {} tokens, which is less than the noise threshold of {}.", &ignored_file.contents.len(), noise),
                    warn_type: WarningType::Fingerprint,
                }
            ]
        );
    }

    #[test]
    fn ignored_files() {
        let noise = 3;
        let guarantee = 3;
        let files = vec![
            File {
                project: "Project 1".into(),
                path: "File 1".into(),
                contents: "aaabbbccc".to_owned(),
            },
            File {
                project: "Project 2".into(),
                path: "File 2".into(),
                contents: "cccxyzaaa".to_owned(),
            },
        ];
        let ignored_files = vec![File {
            project: "Starter Code".into(),
            path: "Starter Code".into(),
            contents: "aaa".to_owned(),
        }];
        let (project_pairs, warnings) = detect_plagiarism(
            noise,
            guarantee,
            TokenizingStrategy::Bytes,
            false,
            0,
            None,
            &files,
            &ignored_files,
        );

        assert!(warnings.is_empty());
        assert_eq!(
            project_pairs,
            vec![ProjectPair {
                project1: "Project 1".into(),
                project2: "Project 2".into(),
                num_matches: 1,
                matches: vec![Match {
                    project1_occurrences: vec![Location {
                        file: "File 1".into(),
                        span: 6..9
                    }],
                    project2_occurrences: vec![Location {
                        file: "File 2".into(),
                        span: 0..3
                    }]
                }]
            }]
        );
    }

    #[test]
    fn common_hashes() {
        let noise = 3;
        let guarantee = 3;
        let files = vec![
            File {
                project: "Project 1".into(),
                path: "File 1".into(),
                contents: "aaabbbccc".to_owned(),
            },
            File {
                project: "Project 2".into(),
                path: "File 2".into(),
                contents: "cccxyzaaa".to_owned(),
            },
            File {
                project: "Project 3".into(),
                path: "File 3".into(),
                contents: "aaa".to_owned(),
            },
            File {
                project: "Project 4".into(),
                path: "File 4".into(),
                contents: "111".to_owned(),
            },
        ];
        let (project_pairs, warnings) = detect_plagiarism(
            noise,
            guarantee,
            TokenizingStrategy::Bytes,
            false,
            0,
            Some(0.75),
            &files,
            &[],
        );

        assert!(warnings.is_empty());
        assert_eq!(
            project_pairs,
            vec![ProjectPair {
                project1: "Project 1".into(),
                project2: "Project 2".into(),
                num_matches: 1,
                matches: vec![Match {
                    project1_occurrences: vec![Location {
                        file: "File 1".into(),
                        span: 6..9
                    }],
                    project2_occurrences: vec![Location {
                        file: "File 2".into(),
                        span: 0..3
                    }]
                }]
            }]
        );
    }
}
