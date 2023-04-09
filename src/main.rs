use anyhow::Context;
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

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
    /// Files and directories containing starter code. Any matches with this code will be ignored.
    #[arg(short, long)]
    ignore: Vec<PathBuf>,
    /// Noise threshold. Matches whose length is less than this value will not be flagged.
    #[arg(short, long, default_value_t = 5)]
    noise: usize,
    /// Guarantee threshold. Matches at least as long as this value are guaranteed to be flagged.
    #[arg(short, long, default_value_t = 10)]
    guarantee: usize,
    /// Tokenizing strategy to use. Can be one of "bytes", "naive", or "relative".
    #[arg(value_enum, short, long, default_value = "bytes")]
    tokenizing_strategy: TokenizingStrategy,
    /// Whether to ignore comments, whitespace, and newlines while tokenizing. This is only supported by the "naive" and
    /// "relative" tokenizing strategies.
    #[arg(short, long, default_value_t = false)]
    ignore_whitespace: bool,
    /// Whether the JSON output should be pretty-printed.
    #[arg(short, long, default_value_t = false)]
    pretty: bool,
    /// Output file.
    #[arg(short, long, default_value = "./fungus-output.json")]
    output_file: PathBuf,
    /// Similarity threshold. Pairs of projects with fewer than this number of matches will not be shown.
    #[arg(short, long, default_value_t = 5)]
    min_matches: usize,
    /// Common code threshold. If the proportion of projects containing some code snippet is greater than this value,
    /// that code will be ignored. The value must be a real number in the range (0, 1].
    #[arg(short, long)]
    common_code_threshold: Option<f64>,
}

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;

    let (documents, mut warnings) = read_projects(&args.root, &args.ignore);

    let (ignored_documents, mut ignored_dir_warnings) = read_starter_code(&args.ignore);
    warnings.append(&mut ignored_dir_warnings);

    let (project_pairs, mut fingerprinting_warnings) = detect_plagiarism(
        args.noise,
        args.guarantee,
        args.tokenizing_strategy,
        args.ignore_whitespace,
        args.min_matches,
        args.common_code_threshold,
        &documents,
        &ignored_documents,
    );
    warnings.append(&mut fingerprinting_warnings);

    let mut output = Output::new(warnings, project_pairs);

    output_results(&mut output, &args.output_file, args.pretty, &args.root)?;

    Ok(())
}

/// Reads, validates, and returns the command-line arguments.
fn parse_args() -> anyhow::Result<Args> {
    let args = Args::parse();

    if !args.root.exists() {
        anyhow::bail!("Projects directory '{}' not found.", args.root.display());
    }
    if !args.root.is_dir() {
        anyhow::bail!(
            "Projects directory '{}' is not a directory.",
            args.root.display()
        );
    }

    for path in args.ignore.iter() {
        if !path.exists() {
            anyhow::bail!("Ignored file or directory '{}' not found.", path.display());
        }
    }

    if args.noise == 0 {
        anyhow::bail!("Noise threshold must be greater than 0.");
    }

    if args.guarantee < args.noise {
        anyhow::bail!("Guarantee threshold must be greater than or equal to noise threshold.");
    }

    if let Some(c) = &args.common_code_threshold {
        if *c <= 0.0 {
            anyhow::bail!("Common hash threshold must be strictly positive.");
        }
        if *c > 1.0 {
            anyhow::bail!("Common hash threshold must be less than or equal to one.");
        }
    }

    if args.ignore_whitespace && args.tokenizing_strategy != TokenizingStrategy::Relative {
        anyhow::bail!(
            "Ignoring whitespace is only supported for the 'relative' tokenizing strategy."
        );
    }

    Ok(args)
}

/// Reads all projects from the given directory. Any paths in `ignore` will be skipped.
fn read_projects(root: &Path, ignore: &[PathBuf]) -> (Vec<File>, Vec<Warning>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for result in WalkDir::new(root).min_depth(1).max_depth(1) {
        match result {
            Err(e) => {
                warnings.push(e.into());
            }
            Ok(entry) => {
                // In case an ignored directory or file is inside the projects directory, skip it.
                // That way we avoid lexing and fingerprinting it twice.
                if ignore.iter().any(|ign| is_same_path(entry.path(), ign)) {
                    continue;
                }

                let (mut fs, mut es) = read_files(entry.path(), ignore);
                files.append(&mut fs);
                warnings.append(&mut es);
            }
        }
    }

    (files, warnings)
}

/// Reads all files containing starter code.
fn read_starter_code(ignore: &[PathBuf]) -> (Vec<File>, Vec<Warning>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for path in ignore {
        let (mut f, mut w) = read_files(path, &[]);
        files.append(&mut f);
        warnings.append(&mut w);
    }

    (files, warnings)
}

/// Reads all the files in the given directory or file. The given directory will be used as the project name.
fn read_files(dir: &Path, files_to_skip: &[PathBuf]) -> (Vec<File>, Vec<Warning>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for result in WalkDir::new(dir) {
        let entry = match result {
            Err(e) => {
                warnings.push(e.into());
                continue;
            }
            Ok(x) => x,
        };
        let path = entry.path();

        if path.is_dir() || files_to_skip.iter().any(|f| is_same_path(path, f)) {
            continue;
        }

        match fs::read_to_string(path) {
            Err(e) => {
                let warning = Warning {
                    file: Some(path.to_owned()),
                    message: e.to_string(),
                    warn_type: WarningType::Input,
                };
                warnings.push(warning);
            }
            Ok(contents) => {
                let file = File::new(dir.to_owned(), path.to_owned(), contents);
                files.push(file);
            }
        }
    }

    (files, warnings)
}

/// Checks if two paths refer to the same file or directory. The two paths may be the same even if their representation
/// is different. For example, `.` and `foo/..` refer to the same directory (assuming `foo` exists).
fn is_same_path(path1: &Path, path2: &Path) -> bool {
    match (path1.canonicalize(), path2.canonicalize()) {
        (Ok(abs_path1), Ok(abs_path2)) => abs_path1 == abs_path2,
        // Just ignore errors here: they can be dealt with elsewhere if necessary
        _ => path1 == path2,
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
