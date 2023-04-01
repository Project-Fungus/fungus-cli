use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use serde::Serialize;

#[derive(Serialize)]
pub struct Output {
    metadata: Metadata,
    pub errors: Vec<Error>,
    pub project_pairs: Vec<ProjectPair>,
}

impl Output {
    pub fn new(errors: Vec<Error>, project_pairs: Vec<ProjectPair>) -> Output {
        let metadata = Metadata {
            num_project_pairs: project_pairs.len(),
        };
        Output {
            metadata,
            errors,
            project_pairs,
        }
    }

    pub fn make_paths_relative_to(
        &mut self,
        root: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for e in self.errors.iter_mut() {
            e.make_paths_relative_to(root)?;
        }
        for pp in self.project_pairs.iter_mut() {
            pp.make_paths_relative_to(root)?;
        }
        Ok(())
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

    pub fn make_paths_relative_to(
        &mut self,
        root: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(f) = &self.file {
            let relative_path = make_path_relative_to(f, root)?;
            self.file = Some(relative_path);
        }
        Ok(())
    }
}

/// Contains information about the similarity of two projects.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProjectPair {
    /// Name of the first project.
    pub project1: PathBuf,
    /// Name of the second project.
    pub project2: PathBuf,
    /// Number of matches detected between the two projects.
    ///
    /// This counts distinct hashes that match between the two projects (e.g., if project 1 contains the hash twice and project 3 has the same hash three times, that is just one match).
    pub num_matches: usize,
    /// Matches between the two projects.
    pub matches: Vec<Match>,
}

impl ProjectPair {
    pub fn make_paths_relative_to(
        &mut self,
        root: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.project1 = make_path_relative_to(&self.project1, root)?;
        self.project2 = make_path_relative_to(&self.project2, root)?;
        for m in self.matches.iter_mut() {
            m.make_paths_relative_to(root)?;
        }
        Ok(())
    }
}

/// Contains information about a specific code snippet that is shared between two projects.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Match {
    /// List of places in which the code snippet appears in project 1.
    pub project1_occurrences: Vec<Location>,
    /// List of places in which the code snipet appears in project 2.
    pub project2_occurrences: Vec<Location>,
}

impl Match {
    fn make_paths_relative_to(&mut self, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
        for location in self.project1_occurrences.iter_mut() {
            location.make_paths_relative_to(root)?;
        }
        for location in self.project2_occurrences.iter_mut() {
            location.make_paths_relative_to(root)?;
        }
        Ok(())
    }
}

/// Absolute reference to a code snippet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Location {
    /// File in which the code snippet is found.
    pub file: PathBuf,
    /// Position of the code snippet within the file (in bytes).
    pub span: Range<usize>,
}

impl Location {
    fn make_paths_relative_to(&mut self, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.file = make_path_relative_to(&self.file, root)?;
        Ok(())
    }
}

fn make_path_relative_to(path: &Path, root: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let canonical_path = path.canonicalize()?;
    let canonical_root = root.canonicalize()?;

    let relative_path = canonical_path.strip_prefix(canonical_root)?;

    Ok(relative_path.to_owned())
}
