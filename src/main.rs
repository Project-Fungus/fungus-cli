use anyhow::Context;
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

use manual_analyzer::{
    detect_plagiarism,
    output::{Error, Output},
    File, TokenizingStrategy,
};

/// A simple copy detection tool for the ARM assembly language.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory in which to search for code.
    root: PathBuf,
    /// Noise threshold. Matches whose length is less than this value will not be flagged.
    #[arg(short, long, default_value_t = 5)]
    noise: usize,
    /// Guarantee threshold. Matches at least as long as this value are guaranteed to be flagged.
    #[arg(short, long, default_value_t = 10)]
    guarantee: usize,
    /// Tokenizing strategy to use. Can be one of "bytes", "naive", or "relative".
    #[arg(value_enum, short, long, default_value = "bytes")]
    tokenizing_strategy: TokenizingStrategy,
    /// Whether the JSON output should be pretty-printed.
    #[arg(short, long, default_value_t = false)]
    pretty: bool,
    /// Output file.
    #[arg(short, long, default_value = "./fungus-output.json")]
    output_file: PathBuf,
    /// Similarity threshold. Pairs of projects with fewer than this number of matches will not be shown.
    #[arg(short, long, default_value_t = 5)]
    min_matches: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.noise == 0 {
        anyhow::bail!("Noise threshold must be greater than 0.");
    }

    if args.guarantee < args.noise {
        anyhow::bail!("Guarantee threshold must be greater than or equal to noise threshold.");
    }

    let (documents, errors) = read_projects(&args.root);

    let project_pairs = detect_plagiarism(
        args.noise,
        args.guarantee,
        args.tokenizing_strategy,
        &documents,
        args.min_matches,
    );
    let output = Output::new(errors, project_pairs);

    output_matches(output, &args.output_file, args.pretty)?;

    Ok(())
}

fn read_projects(root: &Path) -> (Vec<File>, Vec<Error>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();

    for entry in WalkDir::new(root).min_depth(1).max_depth(1) {
        match entry {
            Err(e) => {
                errors.push(Error::from_walkdir(e));
            }
            Ok(x) => {
                let (mut fs, mut es) = read_files(x);
                files.append(&mut fs);
                errors.append(&mut es);
            }
        }
    }

    (files, errors)
}

fn read_files(project: DirEntry) -> (Vec<File>, Vec<Error>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();

    for result in WalkDir::new(project.path()).min_depth(1) {
        let entry = match result {
            Err(e) => {
                errors.push(Error::from_walkdir(e));
                continue;
            }
            Ok(x) => x,
        };

        match try_read_file(&entry) {
            Err(e) => {
                errors.push(e);
            }
            Ok(None) => {
                continue;
            }
            Ok(Some((path, contents))) => {
                let file = File::new(project.path().to_owned(), path, contents);
                files.push(file);
            }
        }
    }

    (files, errors)
}

/// Tries to read a file. Returns the file's path and contents on success, returns `None` if the given `DirEntry` is actually a directory, and returns an error if the operation fails.
fn try_read_file(entry: &DirEntry) -> Result<Option<(PathBuf, String)>, Error> {
    let metadata = match entry.metadata() {
        Err(e) => return Err(Error::from_walkdir(e)),
        Ok(m) => m,
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let path = entry.path().to_owned();
    match fs::read_to_string(&path) {
        Err(e) => Err(Error {
            file: Some(path),
            cause: e.to_string(),
        }),
        Ok(contents) => Ok(Some((path, contents))),
    }
}

fn output_matches(output: Output, output_file: &PathBuf, pretty: bool) -> anyhow::Result<()> {
    let json = if pretty {
        serde_json::to_string_pretty(&output).unwrap()
    } else {
        serde_json::to_string(&output).unwrap()
    };

    fs::write(output_file, json)
        .with_context(|| format!("Failed to write output to \"{}\".", output_file.display()))?;

    println!("Wrote output to \"{}\".", output_file.display());

    Ok(())
}
