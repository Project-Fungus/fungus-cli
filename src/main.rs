use anyhow::Context;
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use manual_analyzer::{
    detect_plagiarism,
    lexing::TokenizingStrategy,
    output::{Output, Warning, WarningType},
    File,
};

/// A simple copy detection tool for the ARM assembly language.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory in which to search for code.
    root: PathBuf,
    /// Files and directories containing starter code. Any matches with this code will be ignored.
    #[arg(long)]
    ignore: Vec<PathBuf>,
    /// Noise threshold. Matches whose length is less than this value will not be flagged.
    #[arg(short, long, default_value_t = 5)]
    noise: usize,
    /// Guarantee threshold. Matches at least as long as this value are guaranteed to be flagged.
    #[arg(short, long, default_value_t = 10)]
    guarantee: usize,
    /// Maximum offset for relative tokens. This argument is not applicable for
    /// non-relative tokens. The default value is `noise - 1`.
    ///
    /// Choosing a very small max offset will probably result in many false
    /// positives. In the extreme case of the max offset being 0, this reduces
    /// to non-relative lexing but with no distinction between registers,
    /// labels, etc. Conversely, choosing a very large max offset will probably
    /// result in many false negatives. In the extreme case of there being no
    /// limit, the algorithm depends on the overall structure of the document
    /// and so no matter how large the match between two projects, there is no
    /// guarantee it will be reported.
    #[arg(long, default_value_t = 0)]
    max_token_offset: usize,
    /// Tokenizing strategy to use. Can be one of "bytes", "naive", or "relative".
    #[arg(value_enum, short, long, default_value = "bytes")]
    tokenizing_strategy: TokenizingStrategy,
    /// Whether to ignore comments, whitespace, and newlines while tokenizing. This is only supported by the "naive" and
    /// "relative" tokenizing strategies.
    #[arg(short, long, default_value_t = false)]
    ignore_whitespace: bool,
    /// Whether to expand matches as much as possible before reporting them.
    #[arg(short, long, default_value_t = true, action = clap::ArgAction::Set)]
    expand_matches: bool,
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
    let (args, mut warnings) = parse_args()?;

    let (documents, mut input_warnings) = read_projects(&args.root, &args.ignore);
    warnings.append(&mut input_warnings);

    let (ignored_documents, mut ignored_dir_warnings) = read_starter_code(&args.ignore);
    warnings.append(&mut ignored_dir_warnings);

    let (project_pairs, mut fingerprinting_warnings) = detect_plagiarism(
        args.noise,
        args.guarantee,
        args.max_token_offset,
        args.tokenizing_strategy,
        args.ignore_whitespace,
        args.expand_matches,
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
fn parse_args() -> anyhow::Result<(Args, Vec<Warning>)> {
    let mut args = Args::parse();
    let mut warnings = Vec::new();

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

    match (args.tokenizing_strategy, args.max_token_offset) {
        (TokenizingStrategy::Relative, 0) => {
            // Default value
            args.max_token_offset = args.noise - 1;
        }
        (TokenizingStrategy::Relative, n) if n < args.noise - 1 => {
            warnings.push(Warning {
                file: None,
                message: "The selected max token offset is very small. This may lead to excessive false positives.".to_owned(),
                warn_type: WarningType::Args,
            });
        }
        (TokenizingStrategy::Relative, _) => {}
        (TokenizingStrategy::Bytes | TokenizingStrategy::Naive, n) if n != 0 => {
            anyhow::bail!("Max token offset must be zero for non-relative tokenizing strategies.");
        }
        (TokenizingStrategy::Bytes | TokenizingStrategy::Naive, _) => {}
    }

    if args.guarantee < args.noise + args.max_token_offset {
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

    if args.ignore_whitespace && args.tokenizing_strategy == TokenizingStrategy::Bytes {
        anyhow::bail!("Ignoring whitespace is not supported for the 'bytes' tokenizing strategy.");
    }

    Ok((args, warnings))
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
