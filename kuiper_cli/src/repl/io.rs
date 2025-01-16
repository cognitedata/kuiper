use std::cmp::min;

use colored::Colorize;
use kuiper_lang::{CompileError, TransformError};

macro_rules! printerr {
    ( $description:expr, $error:expr ) => {
        eprintln!("{} {} {}", "Error:".red(), $description, $error)
    };
}
pub(crate) use printerr;

const WIDTH_THRESHOLD: usize = 70;

fn pretty_print_error_message(
    expression: &str,
    error_span: core::ops::Range<usize>,
    message: String,
) {
    let error_width = error_span.end - error_span.start;

    let offset: usize = if expression.len() < WIDTH_THRESHOLD {
        // Line is short, print the whole thing
        eprintln!("{}", expression.cyan());
        0
    } else {
        // Line is wide, print just an expert containing the error
        if error_width > WIDTH_THRESHOLD {
            // Error is really wide, just print error
            eprintln!("{}", &expression[error_span.clone()].cyan());
            0
        } else {
            // Error is short, include surrounding code
            let mut left_pad = (WIDTH_THRESHOLD - error_width) / 2;
            let mut right_pad = left_pad;

            if error_span.start < left_pad {
                right_pad += left_pad - error_span.start;
                left_pad = 0;
            }

            if error_span.end + right_pad > expression.len() {
                let p = error_span.end + right_pad - expression.len();
                right_pad -= p;
                left_pad += p;
            }
            let mut offset = error_span.start - left_pad;

            let prepend = if offset > 3 {
                offset -= 3;
                "..."
            } else if offset > 0 {
                left_pad += offset;
                offset = 0;
                ""
            } else {
                ""
            };

            let append = if error_span.end + right_pad < expression.len() {
                "..."
            } else {
                ""
            };

            eprintln!(
                "{}{}{}",
                prepend,
                &expression[error_span.start - left_pad..error_span.end + right_pad].cyan(),
                append
            );

            offset
        }
    };

    let indent = error_span.start - offset;

    let message_indent = if indent > min(WIDTH_THRESHOLD / 2, expression.len() / 2) {
        // Right align message if on the error is on the right side of the screen
        if message.len() > indent + error_width {
            0
        } else {
            indent + error_width - message.len()
        }
    } else {
        // Otherwise, left align
        indent
    };

    eprintln!("{}{}", " ".repeat(indent), "^".repeat(error_width).red());
    for line in message.split('\n') {
        eprintln!("{}{}", " ".repeat(message_indent), line.red());
    }
}

pub(crate) fn print_compile_error(expression: &str, error: &CompileError) {
    eprintln!("{} Compilation failed!\n", "Error:".red());

    if let Some(error_span) = error.span() {
        pretty_print_error_message(expression, error_span, error.message());
    } else {
        eprintln!("{}", error.message().red());
    };
}

pub(crate) fn print_transform_error(expression: &str, error: &TransformError) {
    eprintln!("{} Transform failed!\n", "Error:".red());

    if let Some(error_span) = error.span() {
        pretty_print_error_message(expression, error_span, error.message());
    } else {
        eprintln!("{}", error.message().red());
    };
}
