use crate::token::{Token, TokenKind};

pub struct Lexer {
    input: String,
}

impl Lexer {
    pub fn new(input: String) -> Lexer {
        Lexer { input }
    }

    pub fn lex(&self) -> Vec<Token> {
        use TokenKind::*;

        let mut tokens = Vec::new();
        let chars: Vec<char> = self.input.chars().collect();

        let mut current_index: usize = 0;

        loop {
            let token_start = current_index;
            let Some(c) = chars.get(current_index) else {
                break;
            };

            tokens.push(match c {
                c if c.is_whitespace() => {
                    // Peek at the next character
                    while let Some(next_c) = chars.get(current_index + 1) {
                        // If it not whitespace, we have reached the end of the token
                        if !next_c.is_whitespace() {
                            break;
                        }
                        // Keep adding characters to the token
                        current_index += 1;
                    }
                    // Push the complete token
                    Token::new(Whitespace, token_start, current_index)
                }
                '@' => {
                    // Peek at the next character
                    while let Some(next_c) = chars.get(current_index + 1) {
                        // If it is a newline, we have reached the end of the token
                        if next_c == &'\n' {
                            break;
                        }
                        // Keep adding characters to the token
                        current_index += 1;
                    }
                    Token::new(Comment, token_start, current_index)
                }
                ',' => Token::new(Comma, token_start, current_index),
                ':' => Token::new(Colon, token_start, current_index),
                _ => {
                    // Peek at the next character
                    while let Some(next_c) = chars.get(current_index + 1) {
                        // If it is a whitespace or a special character, we have reached the end of the token
                        if next_c.is_whitespace()
                            || next_c == &'@'
                            || next_c == &','
                            || next_c == &':'
                        {
                            break;
                        }
                        // Keep adding characters to the token
                        current_index += 1;
                    }
                    Token::new(Word, token_start, current_index)
                }
            });

            current_index += 1;
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_simple_line() {
        let input = "add r0, r1".to_string();
        let lexer = Lexer::new(input);
        assert_eq!(
            lexer.lex(),
            vec![
                Token::new(TokenKind::Word, 0, 2),
                Token::new(TokenKind::Whitespace, 3, 3),
                Token::new(TokenKind::Word, 4, 5),
                Token::new(TokenKind::Comma, 6, 6),
                Token::new(TokenKind::Whitespace, 7, 7),
                Token::new(TokenKind::Word, 8, 9),
            ]
        )
    }

    #[test]
    fn a_line_with_comments() {
        let input = "add r0, r1 @ This is a comment".to_string();
        let lexer = Lexer::new(input);
        assert_eq!(
            lexer.lex(),
            vec![
                Token::new(TokenKind::Word, 0, 2),
                Token::new(TokenKind::Whitespace, 3, 3),
                Token::new(TokenKind::Word, 4, 5),
                Token::new(TokenKind::Comma, 6, 6),
                Token::new(TokenKind::Whitespace, 7, 7),
                Token::new(TokenKind::Word, 8, 9),
                Token::new(TokenKind::Whitespace, 10, 10),
                Token::new(TokenKind::Comment, 11, 29),
            ]
        )
    }

    #[test]
    fn a_label() {
        let input: String = "label: add r0, r1".to_string();
        let lexer = Lexer::new(input);
        assert_eq!(
            lexer.lex(),
            vec![
                Token::new(TokenKind::Word, 0, 4),
                Token::new(TokenKind::Colon, 5, 5),
                Token::new(TokenKind::Whitespace, 6, 6),
                Token::new(TokenKind::Word, 7, 9),
                Token::new(TokenKind::Whitespace, 10, 10),
                Token::new(TokenKind::Word, 11, 12),
                Token::new(TokenKind::Comma, 13, 13),
                Token::new(TokenKind::Whitespace, 14, 14),
                Token::new(TokenKind::Word, 15, 16),
            ]
        )
    }
}
