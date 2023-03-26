use anyhow::Context;
use clap::Parser;
use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

use manual_analyzer::{detect_plagiarism, File, Match, TokenizingStrategy};

/// A simple copy detection tool for the ARM assembly language.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory in which to search for code.
    projects: PathBuf,
    /// Noise threshold. Matches whose length is less than this value will not be flagged.
    #[arg(short, long, default_value_t = 5)]
    noise: usize,
    /// Guarantee threshold. Matches at least as long as this value are guaranteed to be flagged.
    #[arg(short, long, default_value_t = 10)]
    guarantee: usize,
    /// Tokenizing strategy to use. Can be one of "bytes", "naive", or "relative".
    #[arg(value_enum, short, long, default_value = "bytes")]
    tokenizing_strategy: TokenizingStrategy,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.noise == 0 {
        anyhow::bail!("Noise threshold must be greater than 0.");
    }

    if args.guarantee < args.noise {
        anyhow::bail!("Guarantee threshold must be greater than or equal to noise threshold.");
    }

    let projects = fs::read_dir(&args.projects)
        .with_context(|| format!("Failed to read directory entries at {:?}", &args.projects))?
        .collect::<Result<Vec<_>, _>>()?;

    let project_contents = projects
        .iter()
        .map(get_contents)
        .collect::<Result<Vec<_>, _>>()?;

    // TODO: fingerprint files, not projects (so that the spans are meaningful)
    let documents = project_contents
        .iter()
        .map(|(path, contents)| File::new(path.to_str().unwrap(), path.to_str().unwrap(), contents))
        .collect::<Vec<_>>();
    let document_references = documents.iter().collect::<Vec<_>>();

    let matches = detect_plagiarism(
        args.noise,
        args.guarantee,
        args.tokenizing_strategy,
        &document_references,
    );

    output_matches(matches);

    Ok(())
}

/// Returns the contents of all files in the given directory concatenated
/// together. If the given path is not a directory, then None is returned.
// TODO: Replace with a library like `walkdir`.
fn get_contents(path: &DirEntry) -> anyhow::Result<(PathBuf, String)> {
    let metadata = path
        .metadata()
        .with_context(|| format!("Failed to read directory entry metadata at {path:?}"))?;

    if metadata.is_dir() {
        let mut contents = String::new();
        for child in fs::read_dir(path.path())
            .with_context(|| format!("Failed to read directory entries at {:?}", path.path()))?
        {
            let child = child?;
            let (_, child_contents) = get_contents(&child)?;
            contents += &child_contents;
        }
        Ok((path.path(), contents))
    } else {
        let contents = std::fs::read_to_string(path.path())
            .with_context(|| format!("Failed to parse \"{:?}\" as UTF-8", path.path()))?;
        Ok((path.path(), contents))
    }
}

fn output_matches(matches: Vec<Match>) {
    if matches.is_empty() {
        println!("No matches found.");
    } else {
        let json = serde_json::to_string_pretty(&matches).unwrap();

        println!("{json}");
    }
}
