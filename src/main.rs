use clap::Parser;
use std::{
    fs::{self, DirEntry},
    io,
    path::PathBuf,
};

use manual_analyzer::detect_plagiarism;

/// A simple copy detection tool for the ARM assembly language.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory in which to search for code.
    #[arg(short, long, default_value = ".")]
    root: PathBuf,
    /// Noise threshold. Matches whose length is less than this value will not be flagged.
    #[arg(short, long, default_value_t = 5)]
    noise: usize,
    /// Guarantee threshold. Matches at least as long as this value are guaranteed to be flagged.
    #[arg(short, long, default_value_t = 10)]
    guarantee: usize,
    /// Whether to tokenize the documents before fingerprinting.
    #[arg(short, long)]
    lex: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.noise == 0 {
        anyhow::bail!("Noise threshold must be greater than 0.");
    }

    if args.guarantee < args.noise {
        anyhow::bail!("Guarantee threshold must be greater than or equal to noise threshold.");
    }

    let projects = fs::read_dir(args.root)?.collect::<Result<Vec<_>, _>>()?;

    let project_contents = projects
        .iter()
        .map(get_contents)
        .collect::<Result<Vec<_>, _>>()?;

    let matches = detect_plagiarism(args.noise, args.guarantee, args.lex, &project_contents);

    if matches.is_empty() {
        println!("No matches found.");
    } else {
        println!("The following projects have at least one match:");
        for (i, j) in matches {
            let first = projects[i].path();
            let second = projects[j].path();
            println!("{first:?}, {second:?}");
        }
    }

    Ok(())
}

/// Returns the contents of all files in the given directory concatenated
/// together. If the given path is not a directory, then None is returned.
// TODO: Replace with a library like `walkdir`.
fn get_contents(path: &DirEntry) -> Result<String, io::Error> {
    let metadata = path.metadata()?;

    if metadata.is_dir() {
        let mut contents = String::new();
        for child in fs::read_dir(path.path())? {
            let child = child?;
            let child_contents = get_contents(&child)?;
            contents += &child_contents;
        }
        Ok(contents)
    } else {
        let contents = std::fs::read_to_string(path.path())?;
        Ok(contents)
    }
}
