use std::cmp::max;
use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use fingerprint::Fingerprint;
use identity_hash::IdentityHashMap;
use itertools::{iproduct, Itertools};
use lexing::TokenizingStrategy;
use output::{Location, Match, ProjectPair, Warning, WarningType};

pub mod fingerprint;
pub mod identity_hash;
pub mod lexing;
pub mod match_expansion;
pub mod output;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    project: PathBuf,
    path: PathBuf,
    contents: String,
}

impl File {
    pub fn new(project: PathBuf, path: PathBuf, contents: String) -> File {
        File {
            project,
            path,
            contents,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FileId {
    pub project: PathBuf,
    pub path: PathBuf,
}

impl FileId {
    pub fn new(project: PathBuf, path: PathBuf) -> FileId {
        FileId { project, path }
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
    max_token_offset: usize,
    tokenizing_strategy: TokenizingStrategy,
    ignore_whitespace: bool,
    expand_matches: bool,
    min_matches: usize,
    common_hash_threshold: f64,
    documents: &[File],
    ignored_documents: &[File],
) -> (Vec<ProjectPair>, Vec<Warning>) {
    let mut warnings = Vec::new();

    let mut document_hashes = documents
        .iter()
        .map(|f| {
            (
                FileId::new(f.project.clone(), f.path.clone()),
                lexing::tokenize_and_hash(
                    &f.contents,
                    tokenizing_strategy,
                    ignore_whitespace,
                    max_token_offset,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    let ignored_document_hashes = ignored_documents
        .iter()
        .map(|f| {
            (
                FileId::new(f.project.clone(), f.path.clone()),
                lexing::tokenize_and_hash(
                    &f.contents,
                    tokenizing_strategy,
                    ignore_whitespace,
                    max_token_offset,
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    // Remove the contents of the ignored documents from the input documents
    let ignored_docs_warnings = remove_ignored_documents(
        &mut document_hashes,
        &ignored_document_hashes,
        noise_threshold,
        max_token_offset,
    );

    warnings.extend(ignored_docs_warnings);

    let (document_fingerprints, fingerprinting_warnings) = fingerprint_multiple(
        &document_hashes,
        noise_threshold,
        guarantee_threshold,
        max_token_offset,
    );

    warnings.extend(fingerprinting_warnings);

    // Map hashes to their locations
    let mut hash_locations = build_hash_database(document_fingerprints);

    // Filter out hashes that are common to too many projects
    let num_projects = documents
        .iter()
        .map(|f| &f.project)
        .sorted()
        .dedup()
        .count();

    if common_hash_threshold > 0.0 {
        remove_common_hashes(&mut hash_locations, num_projects, common_hash_threshold);
    }

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
        .into_iter()
        .map(|((p1, p2), matches)| ProjectPair {
            project1: p1.to_owned(),
            project2: p2.to_owned(),
            matches,
        })
        .map(|p| {
            if expand_matches {
                match_expansion::expand_matches(p, &document_hashes)
            } else {
                p
            }
        })
        .filter(|p| p.matches.len() >= min_matches)
        .collect();

    sort_output(&mut project_pairs);

    (project_pairs, warnings)
}

fn remove_ignored_documents(
    document_hashes: &mut HashMap<FileId, Vec<(u64, Range<usize>)>>,
    ignored_document_hashes: &HashMap<FileId, Vec<(u64, Range<usize>)>>,
    noise_threshold: usize,
    max_token_offset: usize,
) -> Vec<Warning> {
    // Discard the fingerprinting warnings from the input documents here since they will always be a
    // subset of the warnings obtained in the second fingerprinting pass when detecting plagiarism.
    let (document_fingerprints, _fingerprinting_warnings) = fingerprint_multiple(
        document_hashes,
        noise_threshold,
        // Choose the fingerprinting parameters so that the window size is 1.
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
        noise_threshold + max_token_offset,
        max_token_offset,
    );

    let (ignored_document_fingerprints, ignored_docs_fingerprinting_warnings) =
        fingerprint_multiple(
            ignored_document_hashes,
            noise_threshold,
            noise_threshold + max_token_offset,
            max_token_offset,
        );

    // Map hashes to their locations
    let hash_locations = build_hash_database(document_fingerprints);

    // Find locations of hashes that are also in the ignored documents
    let mut matches: HashMap<FileId, Vec<Range<usize>>> = HashMap::new();

    for (_, ignored_fingerprint) in ignored_document_fingerprints {
        for (ignored_hash, _) in ignored_fingerprint.spanned_hashes {
            if let Some(locations) = hash_locations.get(&ignored_hash) {
                for (input_file_id, input_doc_span) in locations {
                    if let Some(input_file_matches) = matches.get_mut(input_file_id) {
                        input_file_matches.push(input_doc_span.clone());
                    } else {
                        matches.insert((*input_file_id).clone(), vec![input_doc_span.clone()]);
                    }
                }
            }
        }
    }

    // Turn the byte spans into token spans
    for (file_id, spans) in matches.iter_mut() {
        for span in spans {
            let file_hashes = document_hashes.get(file_id).unwrap();
            span.start = file_hashes
                .iter()
                .position(|(_, token_span)| token_span.start == span.start)
                .unwrap();
            span.end = file_hashes
                .iter()
                .position(|(_, token_span)| token_span.end == span.end)
                .unwrap();
        }
    }

    // Remove the matches from `document_hashes`
    for (file_id, spans) in matches {
        remove_spans_from_vec(document_hashes.get_mut(&file_id).unwrap(), &spans);
    }

    ignored_docs_fingerprinting_warnings
}

// Removes the elements whose indices fall into any of the given spans.
//
// The spans may be in any order and overlap.
fn remove_spans_from_vec<T>(v: &mut Vec<T>, spans: &[Range<usize>]) {
    if spans.is_empty() {
        return;
    }

    // Sort spans
    let mut spans = spans.to_vec();
    spans.sort_unstable_by_key(|s| s.start);

    // Merge overlapping spans
    let mut merged_spans = vec![spans[0].clone()];

    for span in &spans[1..] {
        let last_merged_span = merged_spans.last_mut().unwrap();
        if span.start <= last_merged_span.end {
            last_merged_span.end = max(span.end, last_merged_span.end);
        } else {
            merged_spans.push(span.clone());
        }
    }

    // Remove the spans from the vector
    for span in merged_spans.into_iter().rev() {
        v.drain(span.clone());
    }
}

fn fingerprint_multiple(
    document_hashes: &HashMap<FileId, Vec<(u64, Range<usize>)>>,
    noise_threshold: usize,
    guarantee_threshold: usize,
    max_token_offset: usize,
) -> (Vec<(&FileId, Fingerprint)>, Vec<Warning>) {
    let fingerprint_results = document_hashes.iter().map(|(file_id, hashes)| {
        (
            file_id,
            fingerprint::fingerprint(
                noise_threshold,
                guarantee_threshold,
                max_token_offset,
                hashes,
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

/// Constructs a "hash database" that maps a hash to all the locations in which it was found in the code.
fn build_hash_database<'a, I>(fingerprints: I) -> IdentityHashMap<Vec<(&'a FileId, Range<usize>)>>
where
    I: IntoIterator<Item = (&'a FileId, Fingerprint)>,
{
    let mut hash_locations: IdentityHashMap<Vec<(&'a FileId, Range<usize>)>> =
        IdentityHashMap::default();

    for (file_id, fingerprint) in fingerprints.into_iter() {
        for (hash, span) in fingerprint.spanned_hashes {
            match hash_locations.get_mut(&hash) {
                None => {
                    hash_locations.insert(hash, vec![(file_id, span)]);
                }
                Some(lst) => {
                    lst.push((file_id, span));
                }
            }
        }
    }

    hash_locations
}

fn remove_common_hashes(
    hash_database: &mut IdentityHashMap<Vec<(&FileId, Range<usize>)>>,
    num_projects: usize,
    common_hash_threshold: f64,
) {
    hash_database.retain(|_hash, locations| {
        let num_projects_where_this_hash_occurs = locations
            .iter()
            .map(|(file_id, _)| &file_id.project)
            .sorted()
            .dedup()
            .count();
        (num_projects_where_this_hash_occurs as f64) < (num_projects as f64) * common_hash_threshold
    });
}

/// Converts a set of locations (i.e., identical code snippets) into a set of matches between distinct projects.
fn locations_to_matches<'a>(
    locations: &[(&'a FileId, Range<usize>)],
) -> Vec<(&'a PathBuf, &'a PathBuf, Match)> {
    let grouped_locations = group_locations(locations);

    let mut matches = Vec::new();
    for ((&project_1, project_1_occurrences), (&project_2, project_2_occurrences)) in
        iproduct!(grouped_locations.iter(), grouped_locations.iter())
    {
        // Don't include matches within the same project
        if project_1 >= project_2 {
            continue;
        }

        for (project_1_location, project_2_location) in
            iproduct!(project_1_occurrences, project_2_occurrences)
        {
            let m = Match {
                project_1_location: project_1_location.to_owned(),
                project_2_location: project_2_location.to_owned(),
            };
            matches.push((project_1, project_2, m));
        }
    }

    matches
}

/// Groups a set of locations by project.
fn group_locations<'a>(
    locations: &[(&'a FileId, Range<usize>)],
) -> HashMap<&'a PathBuf, Vec<Location>> {
    let mut grouped_locations: HashMap<&PathBuf, Vec<Location>> = HashMap::default();

    for (file_id, span) in locations {
        let location = Location {
            file: file_id.path.to_owned(),
            span: span.to_owned(),
        };
        match grouped_locations.get_mut(&file_id.project) {
            None => {
                grouped_locations.insert(&file_id.project, vec![location]);
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
    project_pairs.sort_unstable_by_key(|p| p.matches.len());
    project_pairs.reverse();

    for pp in project_pairs {
        pp.matches.sort_unstable_by(|m1, m2| {
            (
                &m1.project_1_location.file,
                m1.project_1_location.span.start,
            )
                .cmp(&(
                    &m2.project_1_location.file,
                    m2.project_1_location.span.start,
                ))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
            0,
            TokenizingStrategy::Bytes,
            false,
            false,
            0,
            0.0,
            &documents,
            &[],
        );

        assert!(warnings.is_empty());
        assert_eq!(
            matches,
            vec![ProjectPair {
                project1: "P1".into(),
                project2: "P2".into(),
                matches: vec![
                    Match {
                        project_1_location: Location {
                            file: "C:/P1/file1.txt".into(),
                            span: 0..3
                        },
                        project_2_location: Location {
                            file: "C:/P2/file.txt".into(),
                            span: 3..6
                        }
                    },
                    Match {
                        project_1_location: Location {
                            file: "C:/P1/file2.txt".into(),
                            span: 0..3
                        },
                        project_2_location: Location {
                            file: "C:/P2/file.txt".into(),
                            span: 3..6
                        }
                    },
                    Match {
                        project_1_location: Location {
                            file: "C:/P1/file2.txt".into(),
                            span: 3..6
                        },
                        project_2_location: Location {
                            file: "C:/P2/file.txt".into(),
                            span: 0..3,
                        },
                    },
                    Match {
                        project_1_location: Location {
                            file: "C:/P1/file2.txt".into(),
                            span: 9..12
                        },
                        project_2_location: Location {
                            file: "C:/P2/file.txt".into(),
                            span: 3..6
                        }
                    },
                    Match {
                        project_1_location: Location {
                            file: "C:/P1/file2.txt".into(),
                            span: 15..18,
                        },
                        project_2_location: Location {
                            file: "C:/P2/file.txt".into(),
                            span: 6..9
                        },
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
            0,
            TokenizingStrategy::Bytes,
            false,
            false,
            5,
            0.0,
            &[file.to_owned()],
            &[ignored_file.to_owned()],
        );

        assert!(project_pairs.is_empty());
        assert_eq!(
            warnings,
            vec![
                Warning {
                    file: Some("Ignored File".into()),
                    message: format!("File could not be fingerprinted because it contains {} tokens, which is less than the noise threshold of {}.", &ignored_file.contents.len(), noise),
                    warn_type: WarningType::Fingerprint,
                },
                Warning {
                    file: Some("File".into()),
                    message: format!("File could not be fingerprinted because it contains {} tokens, which is less than the noise threshold of {}.", &file.contents.len(), noise),
                    warn_type: WarningType::Fingerprint,
                },
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
            0,
            TokenizingStrategy::Bytes,
            false,
            false,
            0,
            0.0,
            &files,
            &ignored_files,
        );

        assert!(warnings.is_empty());
        assert_eq!(
            project_pairs,
            vec![ProjectPair {
                project1: "Project 1".into(),
                project2: "Project 2".into(),
                matches: vec![Match {
                    project_1_location: Location {
                        file: "File 1".into(),
                        span: 6..9
                    },
                    project_2_location: Location {
                        file: "File 2".into(),
                        span: 0..3
                    }
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
            0,
            TokenizingStrategy::Bytes,
            false,
            false,
            0,
            0.75,
            &files,
            &[],
        );

        assert!(warnings.is_empty());
        assert_eq!(
            project_pairs,
            vec![ProjectPair {
                project1: "Project 1".into(),
                project2: "Project 2".into(),
                matches: vec![Match {
                    project_1_location: Location {
                        file: "File 1".into(),
                        span: 6..9
                    },
                    project_2_location: Location {
                        file: "File 2".into(),
                        span: 0..3
                    }
                }]
            }]
        );
    }

    #[test]
    fn limited_relative_offsets() {
        let noise = 8;
        let guarantee = 12;
        let max_token_offset = 4;
        let files = vec![
            File {
                project: "Project 1".into(),
                path: "File 1".into(),
                // The 2nd r1 has an offset of 14
                contents: "mov r1, sp\nfoo\nbar\nsub r0, r2, r0\nadd r0, r1, r2".to_owned(),
            },
            File {
                project: "Project 2".into(),
                path: "File 2".into(),
                // The 2nd r1 has an offset of 12 (different from File 1!)
                contents: "baz\nwaldo\nmov r1, sp\nsub r0, r2, r0\nadd r0, r1, r2".to_owned(),
            },
        ];
        let (project_pairs, warnings) = detect_plagiarism(
            noise,
            guarantee,
            max_token_offset,
            TokenizingStrategy::Relative,
            true,
            true,
            0,
            0.0,
            &files,
            &[],
        );

        assert!(warnings.is_empty());
        assert_eq!(
            project_pairs,
            vec![ProjectPair {
                project1: "Project 1".into(),
                project2: "Project 2".into(),
                matches: vec![Match {
                    project_1_location: Location {
                        file: "File 1".into(),
                        span: 19..48
                    },
                    project_2_location: Location {
                        file: "File 2".into(),
                        span: 21..50
                    }
                }]
            }]
        )
    }
}
