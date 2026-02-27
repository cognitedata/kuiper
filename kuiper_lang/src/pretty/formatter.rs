use core::iter::Peekable;

use alloc::string::ToString;
use logos::{Logos, Span};

use crate::{
    lex::Token,
    pretty::utils::{
        check_closing_token, prettify_comment, raw_token, to_indent_token,
        trim_inter_token_whitespace, IndentNode, IndentNodeKind, PrettyError,
    },
};

pub(super) struct Formatter<'a, T: Iterator<Item = (usize, Span)>> {
    /// The raw input string to format.
    input: &'a str,
    /// The indentation node stack, which keeps track of the history of tokens that may cause indentation.
    stack: crate::Vec<IndentNode>,
    /// The final output.
    output: crate::String,
    /// The current indentation level, in spaces.
    indent: usize,
    /// The current indentation caused by postfix chains.
    postfix_indent: usize,
    /// The number of indent tokens on the current line.
    indent_on_line: usize,
    /// The last end position of the previous token.
    last_end: usize,
    /// The last token processed, used to determine spacing.
    last_token: Option<Token>,
    /// The number of tokens on the current line.
    tokens_on_line: usize,
    /// An iterator over the spans of lines in the input.
    lines: Peekable<T>,
}

const INDENT_SIZE: usize = 4;

impl<'a, T: Iterator<Item = (usize, Span)>> Formatter<'a, T> {
    /// Create a new formatter.
    pub(super) fn new(input: &'a str, lines: Peekable<T>) -> Self {
        Self {
            input,
            stack: alloc::vec![IndentNode {
                kind: IndentNodeKind::Initial,
                line: 0,
                caused_indent: false,
                has_postfix_chain: false,
            }],
            output: crate::String::new(),
            indent: 0,
            postfix_indent: 0,
            indent_on_line: 0,
            last_end: 0,
            last_token: None,
            tokens_on_line: 0,
            lines,
        }
    }

    pub fn run(mut self) -> Result<crate::String, PrettyError> {
        // Iterate over the tokens and process them.
        for (token, token_span) in Token::lexer(self.input).spanned() {
            let token = token?;
            self.process_token(token, token_span)?;
        }

        // Finally, push any remaining whitespace after the last token.
        self.output.push_str(&trim_inter_token_whitespace(
            &self.input[self.last_end..],
            self.last_token.as_ref(),
            None,
        ));

        Ok(self.output)
    }

    fn process_token(&mut self, token: Token, token_span: Span) -> Result<(), PrettyError> {
        let current_line = self.advance_to_line_for_token(&token_span)?;
        self.update_indent_from_token(&token, &token_span, current_line)?;

        // Push any whitespace between the last token and the current one.
        self.output.push_str(&trim_inter_token_whitespace(
            &self.input[self.last_end..token_span.start],
            self.last_token.as_ref(),
            Some(&token),
        ));

        // Check if we need to indent the output for a postfix chain.
        self.update_postfix_indent(&token, self.tokens_on_line == 1);

        // If the token is the first on the line, push indent.
        if self.tokens_on_line == 1 {
            self.output
                .push_str(&" ".repeat(self.indent + self.postfix_indent));
        }

        // Now, push the raw token to the output.
        self.last_end = token_span.end;
        if matches!(token, Token::Comment) {
            // If the token is a comment, we first clean it up specifically.
            // Token formatting is not part of normal inter-token padding, because comment start
            // cannot be its own token. Otherwise, comments would not be allowed to contain invalid tokens.
            self.output
                .push_str(&prettify_comment(raw_token(self.input, token_span)));
        } else {
            self.output.push_str(raw_token(self.input, token_span));
        }
        self.last_token = Some(token);

        Ok(())
    }

    /// Advance the formatter to the line which contains the token given by `token_span`.
    fn advance_to_line_for_token(&mut self, token_span: &Span) -> Result<usize, PrettyError> {
        loop {
            // This should be impossible.
            let Some((line_num, line_span)) = self.lines.peek() else {
                return Err(PrettyError::Pretty(
                    "Token outside of input".to_string(),
                    token_span.clone(),
                ));
            };

            // Check if the start of the token is on the current line.
            if line_span.start <= token_span.start && line_span.end > token_span.start {
                self.tokens_on_line += 1;
                if self.last_end >= line_span.start {
                    self.tokens_on_line += 1;
                }
                break Ok(*line_num);
            }

            self.lines.next();
            self.tokens_on_line = 0;
            if self.indent_on_line > 0 {
                self.indent += INDENT_SIZE;
                self.indent_on_line = 0;
            }
        }
    }

    fn update_indent_from_token(
        &mut self,
        token: &Token,
        token_span: &Span,
        current_line: usize,
    ) -> Result<(), PrettyError> {
        // Is the token an opening indent token?
        if let Some(kind) = to_indent_token(token) {
            // Only the last indent token on each line is responsible for the indent level.
            if let Some(n) = self.stack.last_mut() {
                if n.line == current_line {
                    n.caused_indent = false;
                }
            }
            self.stack.push(IndentNode {
                kind,
                line: current_line,
                caused_indent: true,
                has_postfix_chain: false,
            });
            self.indent_on_line += 1;
        }
        // Is the token a closing indent token?
        if let Some(node) = check_closing_token(&mut self.stack, token, token_span)? {
            if node.line == current_line {
                // If the closing token is on the same line, we just reduce the count of indent tokens on the current line.
                self.indent_on_line -= 1;
            } else {
                // Else, we need to reduce the indent level, if the original node caused an indent.
                if node.caused_indent {
                    self.indent -= INDENT_SIZE;
                }
                if node.has_postfix_chain {
                    self.postfix_indent -= INDENT_SIZE;
                }
            }
        }
        Ok(())
    }

    /// Update indentation for postfix chains. This is almost certainly not the best way to do this,
    /// and we may improve on this in the future with some more concrete cases. Currently, all we do
    /// is potentially add one layer of indentation if a line starts with a period.
    ///
    /// Each block can be indented an additional 4 spaces if it contains a postfix chain, meaning an expression on the form
    /// ```ignore
    /// input
    ///     .foo()
    ///     .bar()
    /// ```
    ///
    /// The complexity comes from the fact that each block can only have one layer of postfix indentation,
    /// and that the postfix chain can be interrupted by certain tokens, like commas.
    ///
    /// To deal with this we track postfix indentation separately. In practice postfix_indent is equal to
    /// 4 * number of nodes in the stack with has_postfix_chain set to true.
    ///
    /// The list of tokens we currently use for interruption may be incomplete.
    fn update_postfix_indent(&mut self, token: &Token, is_first_on_line: bool) {
        // Certain tokens can cause us to enter or exit a postfix chain, check those.
        match token {
            Token::Period if is_first_on_line => {
                if let Some(n) = self.stack.last_mut() {
                    if !n.has_postfix_chain {
                        n.has_postfix_chain = true;
                        self.postfix_indent += INDENT_SIZE;
                    }
                }
            }
            Token::Operator(_) | Token::Colon | Token::SemiColon | Token::Comma => {
                if let Some(n) = self.stack.last_mut() {
                    if n.has_postfix_chain {
                        n.has_postfix_chain = false;
                        self.postfix_indent -= INDENT_SIZE;
                    }
                }
            }
            _ => (),
        };
    }
}
