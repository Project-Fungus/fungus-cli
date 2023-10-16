use std::{
    fmt::Display,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::Context;
use relative_path::RelativePathBuf;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub struct Output {
    pub warnings: Vec<Warning>,
    pub project_pairs: Vec<ProjectPair>,
}

impl Output {
    pub fn new(warnings: Vec<Warning>, project_pairs: Vec<ProjectPair>) -> Output {
        Output {
            warnings,
            project_pairs,
        }
    }

    pub fn make_paths_relative_to(&mut self, root: &Path) -> anyhow::Result<()> {
        for e in self.warnings.iter_mut() {
            e.make_paths_relative_to(root)?;
        }
        for pp in self.project_pairs.iter_mut() {
            pp.make_paths_relative_to(root)?;
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct Warning {
    #[serde(serialize_with = "serialize_path_option")]
    pub file: Option<PathBuf>,
    pub message: String,
    pub warn_type: WarningType,
}

impl Warning {
    fn make_paths_relative_to(&mut self, root: &Path) -> anyhow::Result<()> {
        if let Some(f) = &self.file {
            let relative_path = make_path_relative_to(f, root)?;
            self.file = Some(relative_path);
        }
        Ok(())
    }
}

impl Display for Warning {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let context = match &self.file {
            None => format!("{:?} error", self.warn_type),
            Some(f) => format!("{:?} error in \"{}\"", self.warn_type, f.display()),
        };
        write!(formatter, "{context}:\n  {}", self.message)
    }
}

impl From<walkdir::Error> for Warning {
    fn from(error: walkdir::Error) -> Self {
        Warning {
            file: error.path().map(|p| p.to_owned()),
            message: error.to_string(),
            warn_type: WarningType::Input,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub enum WarningType {
    Input,
    Fingerprint,
}

/// Contains information about the similarity of two projects.
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProjectPair {
    /// Name of the first project.
    #[serde(serialize_with = "serialize_path")]
    pub project1: PathBuf,
    /// Name of the second project.
    #[serde(serialize_with = "serialize_path")]
    pub project2: PathBuf,
    /// Matches between the two projects.
    pub matches: Vec<Match>,
}

impl ProjectPair {
    fn make_paths_relative_to(&mut self, root: &Path) -> anyhow::Result<()> {
        self.project1 = make_path_relative_to(&self.project1, root)?;
        self.project2 = make_path_relative_to(&self.project2, root)?;
        for m in self.matches.iter_mut() {
            m.make_paths_relative_to(root)?;
        }
        Ok(())
    }
}

/// Contains information about a specific code snippet that is shared between two projects.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
pub struct Match {
    /// Location in which the code snippet appears in project 1.
    pub project_1_location: Location,
    /// Location in which the code snippet appears in project 2.
    pub project_2_location: Location,
}

impl Match {
    fn make_paths_relative_to(&mut self, root: &Path) -> anyhow::Result<()> {
        self.project_1_location.make_paths_relative_to(root)?;
        self.project_2_location.make_paths_relative_to(root)?;
        Ok(())
    }
}

/// Absolute reference to a code snippet.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
pub struct Location {
    /// File in which the code snippet is found.
    #[serde(serialize_with = "serialize_path")]
    pub file: PathBuf,
    /// Position of the code snippet within the file (in bytes).
    pub span: Range<usize>,
}

impl Location {
    fn make_paths_relative_to(&mut self, root: &Path) -> anyhow::Result<()> {
        self.file = make_path_relative_to(&self.file, root)?;
        Ok(())
    }
}

fn make_path_relative_to(path: &Path, root: &Path) -> anyhow::Result<PathBuf> {
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("Failed to make path '{}' absolute.", path.display()))?;
    let canonical_root = root.canonicalize().with_context(|| {
        format!(
            "Failed to make projects directory path '{}' absolute.",
            &root.display()
        )
    })?;

    let relative_path = canonical_path
        .strip_prefix(&canonical_root)
        .with_context(|| {
            format!(
                "Failed to strip prefix '{}' from '{}'.",
                &canonical_root.display(),
                &canonical_path.display()
            )
        })?;

    Ok(relative_path.to_owned())
}

/// Serializes an `Option<PathBuf>` using `serialize_path`.
fn serialize_path_option<S>(value: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        None => serializer.serialize_none(),
        Some(p) => serialize_path(p, serializer),
    }
}

/// Serializes a `PathBuf`.
///
/// The `relative-path` crate is used to ensure the path separator is always '/'.
fn serialize_path<S>(value: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let relative_path = match RelativePathBuf::from_path(value) {
        Err(_) => {
            return Err(serde::ser::Error::custom(
                "failed to convert PathBuf to RelativePathBuf",
            ))
        }
        Ok(x) => x,
    };
    let path_str = format!("{relative_path}");
    serializer.serialize_str(&path_str)
}
