use std::ops::Range;

use crate::lexing::naive::Token as NaiveToken;
use crate::lexing::relative::Token as RelativeToken;

/// Removes whitespace, comments, and newline tokens from the given token stream, updating the offsets of RelativeSymbol
/// tokens as necessary.
pub fn remove_whitespace_relative(
    tokens: Vec<(RelativeToken, Range<usize>)>,
) -> Vec<(RelativeToken, Range<usize>)> {
    // For each index in tokens, we store whether or not a whitespace token was removed.
    let mut removed = Vec::new();

    fn tokens_removed_in_last_n_tokens(removed: &[bool], n: usize) -> usize {
        removed.iter().rev().take(n).filter(|x| **x).count()
    }

    tokens
        .into_iter()
        .filter_map(|(token, range)| match token {
            // Remove whitespace, comments, and newline tokens
            RelativeToken::Whitespace | RelativeToken::Newline | RelativeToken::Comment(_) => {
                removed.push(true);
                None
            }
            // Adjust offset of RelativeSymbol tokens
            RelativeToken::RelativeSymbol(offset) => {
                let tokens_removed = if offset == 0 {
                    0
                } else {
                    tokens_removed_in_last_n_tokens(&removed, offset - 1)
                };
                removed.push(false);
                Some((
                    RelativeToken::RelativeSymbol(offset - tokens_removed),
                    range,
                ))
            }
            // Keep other tokens as is
            _ => {
                removed.push(false);
                Some((token, range))
            }
        })
        .collect()
}

/// Removes whitespace, comments, and newline tokens from the given token stream.
pub fn remove_whitespace_naive(
    tokens: Vec<(NaiveToken, Range<usize>)>,
) -> Vec<(NaiveToken, Range<usize>)> {
    tokens
        .into_iter()
        .filter(|(token, _)| {
            !matches!(
                token,
                NaiveToken::Whitespace | NaiveToken::Newline | NaiveToken::Comment(_)
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexing::relative::Token as RelativeToken;

    #[test]
    fn remove_whitespace_relative_works() {
        let original_tokens = vec![
            (RelativeToken::RelativeSymbol(0), 0..2),
            (RelativeToken::Comma, 2..3),
            (RelativeToken::Whitespace, 3..4),
            (RelativeToken::RelativeSymbol(3), 4..6),
            (RelativeToken::RelativeSymbol(1), 6..8),
            (RelativeToken::Comment("test"), 8..9),
            (RelativeToken::Newline, 9..10),
            (RelativeToken::RelativeSymbol(0), 10..12),
            (RelativeToken::RelativeSymbol(4), 12..14),
        ];
        let expected_tokens = vec![
            (RelativeToken::RelativeSymbol(0), 0..2),
            (RelativeToken::Comma, 2..3),
            (RelativeToken::RelativeSymbol(2), 4..6),
            (RelativeToken::RelativeSymbol(1), 6..8),
            (RelativeToken::RelativeSymbol(0), 10..12),
            (RelativeToken::RelativeSymbol(2), 12..14),
        ];
        let actual_tokens = remove_whitespace_relative(original_tokens);
        assert_eq!(actual_tokens, expected_tokens);
    }

    #[test]
    fn remove_whitespace_naive_works() {
        let original_tokens = vec![
            (NaiveToken::Symbol("test".to_owned()), 0..4),
            (NaiveToken::Whitespace, 4..5),
            (NaiveToken::Newline, 5..6),
            (NaiveToken::Comment("test"), 6..7),
            (NaiveToken::Symbol("test".to_owned()), 7..11),
        ];
        let expected_tokens = vec![
            (NaiveToken::Symbol("test".to_owned()), 0..4),
            (NaiveToken::Symbol("test".to_owned()), 7..11),
        ];
        let actual_tokens = remove_whitespace_naive(original_tokens);
        assert_eq!(actual_tokens, expected_tokens);
    }
}
