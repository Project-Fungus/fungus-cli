use std::{collections::HashMap, iter::Peekable};

use logos::Lexer;

use super::Token::{self, *};

pub fn parse<'source>(lexer: Lexer<'source, Token<'source>>) -> Vec<Token<'source>> {
    Parser::new(lexer).parse()
}

struct Parser<'source> {
    lexer: Peekable<Lexer<'source, Token<'source>>>,
    result: Vec<Token<'source>>,
    token_count: usize,
    // Maps symbol names to the last token index at which they were encountered
    symbol_occurrences: HashMap<String, usize>,
}

impl<'source> Parser<'source> {
    #[inline]
    fn new(lexer: Lexer<'source, Token<'source>>) -> Self {
        Self {
            lexer: lexer.peekable(),
            result: Vec::new(),
            token_count: 0,
            symbol_occurrences: HashMap::new(),
        }
    }

    #[inline]
    fn parse(mut self) -> Vec<Token<'source>> {
        while self.peek().is_some() {
            self.parse_statement()
        }

        self.result
    }

    #[inline]
    fn next(&mut self) -> Option<Token<'source>> {
        let t = self.lexer.next();
        self.token_count += 1;
        t
    }

    #[inline]
    fn peek(&mut self) -> Option<&Token<'source>> {
        self.lexer.peek()
    }

    #[inline]
    fn relative_symbol(&mut self, symbol: String) -> Token<'source> {
        // Return a `RelativeSymbol` token with the number of tokens since the last occurrence of the symbol
        // or 0 if this is the first occurrence of the symbol
        let relative_symbol = match self.symbol_occurrences.get(&symbol) {
            Some(&index) => RelativeSymbol(self.token_count - index),
            None => RelativeSymbol(0),
        };
        self.symbol_occurrences.insert(symbol, self.token_count);
        relative_symbol
    }

    #[inline]
    fn parse_statement(&mut self) {
        // Each statement (delimited by newlines) begins with zero or more labels, followed by a "key symbol" which can be
        // either an instruction or a directive.
        // Empty statements are allowed.

        // Replace labels, registers, and string literals with `RelativeSymbol` tokens
        // Replace instructions with `Instruction` tokens

        // Parse zero or more labels followed by a key symbol
        while let Some(t) = self.next() {
            match t {
                Newline => {
                    self.result.push(t);
                    return;
                }
                Symbol(s) => {
                    // If the next token is a colon, this is a label, keep looking for a key symbol
                    if let Some(Colon) = self.peek() {
                        let relative_symbol = self.relative_symbol(s);
                        self.result.push(relative_symbol);
                    } else {
                        // This is an instruction, stop looking for a key symbol
                        self.result.push(Instruction(s));
                        break;
                    }
                }
                // Directives are also key symbols
                Directive(_) => {
                    self.result.push(t);
                    break;
                }
                // All other tokens, even syntactically invalid ones are ignored and returned without modifications
                t => {
                    self.result.push(t);
                }
            }
        }

        // Keep parsing the end of the statement until the next newline
        while let Some(t) = self.next() {
            match t {
                Newline => {
                    self.result.push(t);
                    return;
                }
                Symbol(s) => {
                    let relative_symbol = self.relative_symbol(s);
                    self.result.push(relative_symbol);
                }
                t => {
                    self.result.push(t);
                }
            }
        }
    }
}
