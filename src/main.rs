use clap::Parser;
use std::{
    fs::{self, DirEntry},
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let projects = fs::read_dir(args.root)?
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let project_contents = projects
        .iter()
        .map(|x| get_contents(&x))
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();

    let project_contents_borrowed = project_contents
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>();

    let mut matches = detect_plagiarism(args.noise, args.guarantee, &project_contents_borrowed);

    matches.sort();
    matches.dedup();

    if matches.len() == 0 {
        println!("No matches found.");
    } else {
        println!("The following projects have at least one match:");
        for (i, j) in matches.iter() {
            let first = projects.get(*i).unwrap().path();
            let second = projects.get(*j).unwrap().path();
            println!("{}, {}", first.to_str().unwrap(), second.to_str().unwrap());
        }
    }

    Ok(())
}

/// Returns the contents of all files in the given directory concatenated
/// together. If the given path is not a directory, then None is returned.
fn get_contents(path: &DirEntry) -> Result<String, Box<dyn std::error::Error>> {
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
