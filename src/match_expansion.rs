use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use crate::{
    output::{Location, Match, ProjectPair},
    FileId,
};

pub fn expand_matches(
    pair: ProjectPair,
    document_hashes: &HashMap<FileId, Vec<(u64, Range<usize>)>>,
) -> ProjectPair {
    // For every match, expand the match as much as possible.
    // Store the expanded matches in a hash set to avoid duplicates.
    let mut expanded_matches = HashSet::new();

    for Match {
        project_1_location,
        project_2_location,
    } in pair.matches
    {
        let file_1_id = FileId::new(pair.project1.clone(), project_1_location.file.clone());
        let file_2_id = FileId::new(pair.project2.clone(), project_2_location.file.clone());

        let file_1_hashed_tokens = &document_hashes[&file_1_id];
        let file_2_hashed_tokens = &document_hashes[&file_2_id];

        let mut location_1_match_span = project_1_location.span;
        let mut location_2_match_span = project_2_location.span;

        // Find the start of the match in each file
        let mut location_1_start = file_1_hashed_tokens
            .iter()
            .position(|(_, range)| range.start == location_1_match_span.start)
            .unwrap();
        let mut location_2_start = file_2_hashed_tokens
            .iter()
            .position(|(_, range)| range.start == location_2_match_span.start)
            .unwrap();

        // Expand the match upwards (towards the start of the file) as much as possible
        while location_1_start > 0
            && location_2_start > 0
            && file_1_hashed_tokens[location_1_start - 1].0
                == file_2_hashed_tokens[location_2_start - 1].0
        {
            location_1_start -= 1;
            location_2_start -= 1;
        }

        location_1_match_span.start = file_1_hashed_tokens[location_1_start].1.start;
        location_2_match_span.start = file_2_hashed_tokens[location_2_start].1.start;

        // Find the end of the match in each file
        let mut location_1_end = file_1_hashed_tokens
            .iter()
            .rposition(|(_t, range)| range.end == location_1_match_span.end)
            .unwrap();
        let mut location_2_end = file_2_hashed_tokens
            .iter()
            .rposition(|(_t, range)| range.end == location_2_match_span.end)
            .unwrap();

        // Expand the match downwards (towards the end of the file) as much as possible
        while location_1_end < file_1_hashed_tokens.len() - 1
            && location_2_end < file_2_hashed_tokens.len() - 1
            && file_1_hashed_tokens[location_1_end + 1].0
                == file_2_hashed_tokens[location_2_end + 1].0
        {
            location_1_end += 1;
            location_2_end += 1;
        }

        location_1_match_span.end = file_1_hashed_tokens[location_1_end].1.end;
        location_2_match_span.end = file_2_hashed_tokens[location_2_end].1.end;

        // Store the expanded match
        expanded_matches.insert(Match {
            project_1_location: Location {
                file: project_1_location.file.clone(),
                span: location_1_match_span,
            },
            project_2_location: Location {
                file: project_2_location.file.clone(),
                span: location_2_match_span,
            },
        });
    }

    ProjectPair {
        project1: pair.project1,
        project2: pair.project2,
        matches: expanded_matches.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn expands_incomplete_matches() {
        let document_hashes: HashMap<FileId, Vec<(u64, Range<usize>)>> = HashMap::from([
            (
                FileId::new("p1".into(), "f1".into()),
                vec![(1, 0..1), (2, 1..2), (3, 2..3)],
            ),
            (
                FileId::new("p2".into(), "f2".into()),
                vec![(1, 0..1), (2, 1..2), (3, 2..3)],
            ),
        ]);

        let project_pair = ProjectPair {
            project1: "p1".into(),
            project2: "p2".into(),
            matches: vec![Match {
                project_1_location: Location {
                    file: "f1".into(),
                    span: 1..2,
                },
                project_2_location: Location {
                    file: "f2".into(),
                    span: 1..2,
                },
            }],
        };

        assert_eq!(
            expand_matches(project_pair, &document_hashes),
            ProjectPair {
                project1: "p1".into(),
                project2: "p2".into(),
                matches: vec![Match {
                    project_1_location: Location {
                        file: "f1".into(),
                        span: 0..3,
                    },
                    project_2_location: Location {
                        file: "f2".into(),
                        span: 0..3,
                    },
                },]
            }
        );
    }

    #[test]
    fn does_not_expand_complete_matches() {
        let document_hashes: HashMap<FileId, Vec<(u64, Range<usize>)>> = HashMap::from([
            (
                FileId::new("p1".into(), "f1".into()),
                vec![(1, 0..1), (2, 1..2), (3, 2..3)],
            ),
            (
                FileId::new("p2".into(), "f2".into()),
                vec![(3, 0..1), (2, 1..2), (1, 2..3)],
            ),
        ]);

        let project_pair = ProjectPair {
            project1: "p1".into(),
            project2: "p2".into(),
            matches: vec![Match {
                project_1_location: Location {
                    file: "f1".into(),
                    span: 1..2,
                },
                project_2_location: Location {
                    file: "f2".into(),
                    span: 1..2,
                },
            }],
        };

        assert_eq!(
            expand_matches(project_pair, &document_hashes),
            ProjectPair {
                project1: "p1".into(),
                project2: "p2".into(),
                matches: vec![Match {
                    project_1_location: Location {
                        file: "f1".into(),
                        span: 1..2,
                    },
                    project_2_location: Location {
                        file: "f2".into(),
                        span: 1..2,
                    },
                },]
            }
        );
    }
}
