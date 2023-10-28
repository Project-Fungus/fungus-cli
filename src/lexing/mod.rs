use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use rustc_hash::FxHasher;

mod naive;
mod preprocessing;
mod relative;

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum TokenizingStrategy {
    /// Do not tokenize the input. Instead, process the input as a sequence of bytes.
    Bytes,
    /// Tokenize the input using a best-effort, naive GNU ARMv7 assembly tokenizer.
    Naive,
    /// Tokenize the input using a more conservative and transformation-resistant GNU ARM assembly tokenizer.
    ///
    /// This tokenizer represents symbols using relative offsets from their last occurrence in the token sequence.
    /// This requires an additional pass over the input to compute the offsets and identify key symbols
    /// (i.e. instructions and directives).
    Relative,
}

pub fn tokenize_and_hash(
    string: &str,
    tokenizing_strategy: TokenizingStrategy,
    ignore_whitespace: bool,
) -> Vec<(u64, Range<usize>)> {
    match tokenizing_strategy {
        TokenizingStrategy::Bytes => {
            // Use bytes instead of chars since it shouldn't affect the result and is faster.
            let characters = string.as_bytes();
            characters
                .iter()
                .enumerate()
                .map(|(i, &c)| (c, i..i + 1))
                .map(|(c, span)| (hash_token(c), span))
                .collect()
        }
        TokenizingStrategy::Naive => {
            let mut tokens = naive::lex(string);
            if ignore_whitespace {
                tokens = preprocessing::whitespace_removal::remove_whitespace_naive(tokens);
            }
            tokens
                .into_iter()
                .map(|(t, span)| (hash_token(t), span))
                .collect()
        }
        TokenizingStrategy::Relative => {
            let mut tokens = relative::lex(string);
            if ignore_whitespace {
                tokens = preprocessing::whitespace_removal::remove_whitespace_relative(tokens);
            }
            tokens
                .into_iter()
                .map(|(t, span)| (hash_token(t), span))
                .collect()
        }
    }
}

fn hash_token<T: Hash>(token: T) -> u64 {
    // IMPORTANT: create a new hasher each time because hasher.finish() does NOT
    // clear the hasher, it only returns the hash.
    let mut hasher = FxHasher::default();
    token.hash(&mut hasher);
    hasher.finish()
}
