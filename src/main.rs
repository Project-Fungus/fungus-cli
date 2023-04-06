use anyhow::Context;
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

use manual_analyzer::{
    detect_plagiarism,
    output::{Output, Warning, WarningType},
    File, TokenizingStrategy,
};

/// A simple copy detection tool for the ARM assembly language.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory in which to search for code.
    root: PathBuf,
    /// Directory containing starter code. Any matches with this code will be ignored.
    #[arg(short, long)]
    ignore: Option<PathBuf>,
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
    /// Common hash threshold. If the proportion of projects containing some hash is greater than this value,
    /// that hash will be ignored.
    #[arg(short, long)]
    common_hash_threshold: Option<f64>,
}

fn main() -> anyhow::Result<()> {
    let args = get_valid_args()?;

    let (documents, mut warnings) = read_projects(&args.root, &args.ignore);

    let (ignored_documents, mut ignored_dir_warnings) = match args.ignore {
        None => (Vec::new(), Vec::new()),
        Some(ign) => read_files(&ign),
    };
    warnings.append(&mut ignored_dir_warnings);

    let (project_pairs, mut fingerprinting_warnings) = detect_plagiarism(
        args.noise,
        args.guarantee,
        args.tokenizing_strategy,
        args.min_matches,
        args.common_hash_threshold,
        &documents,
        &ignored_documents,
    );
    warnings.append(&mut fingerprinting_warnings);

    let mut output = Output::new(warnings, project_pairs);

    output_results(&mut output, &args.output_file, args.pretty, &args.root)?;

    Ok(())
}

/// Reads, validates, and returns the command-line arguments.
fn get_valid_args() -> anyhow::Result<Args> {
    let args = Args::parse();

    if !args.root.exists() {
        anyhow::bail!("Projects directory '{}' not found.", args.root.display());
    }

    if let Some(ign) = &args.ignore {
        if !ign.exists() {
            anyhow::bail!("Starter code directory '{}' not found.", ign.display());
        }
    }

    if args.noise == 0 {
        anyhow::bail!("Noise threshold must be greater than 0.");
    }

    if args.guarantee < args.noise {
        anyhow::bail!("Guarantee threshold must be greater than or equal to noise threshold.");
    }

    if let Some(c) = &args.common_hash_threshold {
        if *c <= 0.0 {
            anyhow::bail!("Common hash threshold must be strictly positive.");
        }
        if *c > 1.0 {
            anyhow::bail!("Common hash threshold must be less than or equal to one.");
        }
    }

    Ok(args)
}

/// Reads all projects from the given directory. The `ignore` directory will be skipped.
fn read_projects(root: &Path, ignore: &Option<PathBuf>) -> (Vec<File>, Vec<Warning>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(root).min_depth(1).max_depth(1) {
        match (entry, ignore) {
            (Err(e), _) => {
                warnings.push(e.into());
            }
            // TODO: Check if equality works the way I expect it to here
            (Ok(x), Some(ign)) if x.path() == ign => {
                continue;
            }
            (Ok(x), _) => {
                let (mut fs, mut es) = read_files(x.path());
                files.append(&mut fs);
                warnings.append(&mut es);
            }
        }
    }

    (files, warnings)
}

fn read_files(project: &Path) -> (Vec<File>, Vec<Warning>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for result in WalkDir::new(project).min_depth(1) {
        let entry = match result {
            Err(e) => {
                warnings.push(e.into());
                continue;
            }
            Ok(x) => x,
        };

        match try_read_file(&entry) {
            Err(e) => {
                warnings.push(e);
            }
            Ok(None) => {
                continue;
            }
            Ok(Some((path, contents))) => {
                let file = File::new(project.to_owned(), path, contents);
                files.push(file);
            }
        }
    }

    (files, warnings)
}

/// Tries to read a file. Returns the file's path and contents on success, returns `None` if the given `DirEntry` is actually a directory, and returns a warning if the operation fails.
fn try_read_file(entry: &DirEntry) -> Result<Option<(PathBuf, String)>, Warning> {
    let metadata = match entry.metadata() {
        Err(e) => return Err(e.into()),
        Ok(m) => m,
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let path = entry.path().to_owned();
    match fs::read_to_string(&path) {
        Err(e) => Err(Warning {
            file: Some(path),
            message: e.to_string(),
            warn_type: WarningType::Input,
        }),
        Ok(contents) => Ok(Some((path, contents))),
    }
}

fn output_results(
    output: &mut Output,
    output_file: &Path,
    pretty: bool,
    root: &Path,
) -> anyhow::Result<()> {
    output
        .make_paths_relative_to(root)
        .with_context(|| "Failed to make paths relative to the projects directory.")?;

    eprintln!("{} warnings.", output.warnings.len());
    if !output.warnings.is_empty() {
        for w in output.warnings.iter() {
            eprintln!("{w}");
        }
        eprintln!();
    }

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
