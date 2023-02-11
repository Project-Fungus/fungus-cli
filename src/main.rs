use std::path::PathBuf;

use clap::Parser;
use manual_analyzer::lexer::Lexer;

/// A simple lexer for the ARM assembly language.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The input file to lex.
    file: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input = std::fs::read_to_string(args.file)?;
    let lexer = Lexer::new(input);
    println!("{:#?}", lexer.lex());
    
    Ok(())
}
