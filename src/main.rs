use anyhow::Context;
use clap::Parser;
use serde::Serialize;
use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

use manual_analyzer::{detect_plagiarism, File, ProjectPair, TokenizingStrategy};

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
    /// Whether the JSON output should be pretty-printed
    #[arg(short, long, default_value_t = false)]
    pretty: bool,
    /// Output file
    #[arg(short, long, default_value = "./fungus-output.json")]
    output_file: PathBuf,
}

#[derive(Serialize)]
struct Output<'a> {
    project_pairs: Vec<ProjectPair<'a>>,
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

    // TODO: fingerprint files, not projects (so that the spans are meaningful when there are multiple files per project)
    let documents = project_contents
        .iter()
        .map(|(path, contents)| File::new(path.to_str().unwrap(), path.to_owned(), contents))
        .collect::<Vec<_>>();
    let document_references = documents.iter().collect::<Vec<_>>();

    let project_pairs = detect_plagiarism(
        args.noise,
        args.guarantee,
        args.tokenizing_strategy,
        &document_references,
    );

    let output = Output { project_pairs };

    output_matches(output, &args.output_file, args.pretty)?;

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
