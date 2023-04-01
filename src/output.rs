use std::{ops::Range, path::PathBuf};

use serde::Serialize;

#[derive(Serialize)]
pub struct Output<'a> {
    metadata: Metadata,
    pub errors: Vec<Error>,
    pub project_pairs: Vec<ProjectPair<'a>>,
}

impl<'a> Output<'a> {
    pub fn new(errors: Vec<Error>, project_pairs: Vec<ProjectPair<'a>>) -> Output {
        let metadata = Metadata {
            num_project_pairs: project_pairs.len(),
        };
        Output {
            metadata,
            errors,
            project_pairs,
        }
    }
}

#[derive(Serialize)]
struct Metadata {
    num_project_pairs: usize,
}

#[derive(Serialize)]
pub struct Error {
    pub file: Option<PathBuf>,
    pub cause: String,
}

impl Error {
    pub fn from_walkdir(error: walkdir::Error) -> Error {
        Error {
            file: error.path().map(|p| p.to_owned()),
            cause: error.to_string(),
        }
    }
}

/// Contains information about the similarity of two projects.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProjectPair<'a> {
    /// Name of the first project.
    pub project1: &'a PathBuf,
    /// Name of the second project.
    pub project2: &'a PathBuf,
    /// Number of matches detected between the two projects.
    ///
    /// This counts distinct hashes that match between the two projects (e.g., if project 1 contains the hash twice and project 3 has the same hash three times, that is just one match).
    pub num_matches: usize,
    /// Matches between the two projects.
    pub matches: Vec<Match>,
}

/// Contains information about a specific code snippet that is shared between two projects.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Match {
    /// List of places in which the code snippet appears in project 1.
    pub project1_occurrences: Vec<Location>,
    /// List of places in which the code snipet appears in project 2.
    pub project2_occurrences: Vec<Location>,
}

/// Absolute reference to a code snippet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Location {
    /// File in which the code snippet is found.
    pub file: PathBuf,
    /// Position of the code snippet within the file (in bytes).
    pub span: Range<usize>,
}
