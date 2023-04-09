use std::ops::Range;

use crate::lexing::relative::Token as RelativeToken;

/// Removes whitespace, comments, and newline tokens from the given token stream, updating the offsets of RelativeSymbol
/// tokens as necessary.
pub fn remove_whitespace_relative(
    tokens: Vec<(RelativeToken, Range<usize>)>,
) -> Vec<(RelativeToken, Range<usize>)> {
    // For each index in tokens, we store whether or not a whitespace token was removed.
    let mut tokens_removed = Vec::new();

    fn get_tokens_removed_in_last_n_tokens(tokens_removed: &[bool], n: usize) -> usize {
        tokens_removed.iter().rev().take(n).filter(|x| **x).count()
    }

    tokens
        .into_iter()
        .filter_map(|(token, range)| match token {
            // Remove whitespace, comments, and newline tokens
            RelativeToken::Whitespace | RelativeToken::Newline | RelativeToken::Comment(_) => {
                tokens_removed.push(true);
                None
            }
            // Adjust offset of RelativeSymbol tokens
            RelativeToken::RelativeSymbol(offset) => {
                let tokens_removed_since_last_occurrence = if offset == 0 {
                    0
                } else {
                    get_tokens_removed_in_last_n_tokens(&tokens_removed, offset - 1)
                };
                tokens_removed.push(false);
                Some((
                    RelativeToken::RelativeSymbol(offset - tokens_removed_since_last_occurrence),
                    range,
                ))
            }
            // Keep other tokens as is
            _ => {
                tokens_removed.push(false);
                Some((token, range))
            }
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
}
