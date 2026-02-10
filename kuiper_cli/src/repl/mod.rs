mod cmd_helper;
mod io;
mod macros;
mod magic;

use std::collections::HashMap;
use std::time::Instant;

use colored::Colorize;
use io::{print_compile_error, print_transform_error, printerr};
use kuiper_lang::compile_expression;

use macros::Macro;
use regex::Regex;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};
use serde_json::Value;

use crate::repl::magic::apply_magic_function;

pub fn repl(verbose_log: bool) {
    let mut data = Vec::new();
    let mut index = 0usize;
    let mut inputs = Vec::<String>::new();

    let editor_config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let mut readlines = Editor::with_config(editor_config).unwrap();
    readlines.set_helper(Some(cmd_helper::KuiperHelper::new()));

    let mut history_path = dirs::home_dir().unwrap();
    history_path.push(".kuiper_history");

    let macro_pattern = Regex::new(r"#.*?;").unwrap();
    let mut macro_defs = HashMap::new();

    let _ = readlines.load_history(&history_path);

    println!("Kuiper REPL version {}", env!("CARGO_PKG_VERSION"));
    println!("Type /help for a list of available commands. Press ctrl-D to exit.");
    println!();

    let prompt = "kuiper> ".blue().bold().to_string();

    'repl: loop {
        let line = readlines.readline(&prompt);

        match line {
            Ok(mut expression) => {
                if expression.trim().is_empty() {
                    continue;
                }

                let _ = readlines.add_history_entry(expression.as_str());

                if expression.starts_with('/') {
                    match apply_magic_function(
                        expression,
                        &mut data,
                        &mut inputs,
                        &mut index,
                        &mut macro_defs,
                    ) {
                        magic::ReplResult::Continue => {
                            println!();
                            continue;
                        }
                        magic::ReplResult::Stop => break,
                    }
                }

                // Strip off all macro definitions from the expression, store in the macro map.
                while let Some(m) = macro_pattern.find(&expression) {
                    match Macro::from_expression(m.as_str()) {
                        Ok(parsed) => {
                            macro_defs.insert(parsed.name.clone(), parsed);
                            expression = expression.replace(m.as_str(), "");
                        }
                        Err(error_message) => {
                            printerr!("Internal error:", error_message);
                            continue 'repl;
                        }
                    }
                }
                if expression.trim().is_empty() {
                    // If expression is empty now, it means we only got macro defs. They are stored,
                    // there's nothing else to do
                    continue;
                }

                // Re-add all macro definitions
                let formatted_macro_defs = macro_defs
                    .values()
                    .fold("".to_string(), |acc, e| format!("{e} {acc}"));
                expression = format!("{formatted_macro_defs}{expression}");

                let chunk_id = format!("out{index}");
                let compile_start = Instant::now();
                let res = compile_expression(
                    &expression,
                    &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
                );
                if verbose_log {
                    println!(
                        "Compiled in {} ms",
                        compile_start.elapsed().as_micros() as f64 / 1000.0
                    );
                }

                let expr = match res {
                    Ok(x) => x,
                    Err(e) => {
                        print_compile_error(&expression, &e);
                        println!();
                        continue;
                    }
                };

                let run_start = Instant::now();
                let res = expr.run(data.iter());
                if verbose_log {
                    println!(
                        "Run in {} ms",
                        run_start.elapsed().as_micros() as f64 / 1000.0
                    );
                }

                match res {
                    Ok(x) => {
                        let value = x.into_owned();
                        let line = match &value {
                            Value::Object(_) | Value::Array(_) => {
                                let compact = value.to_string();
                                if compact.len() > 50 {
                                    serde_json::to_string_pretty(&value).unwrap_or(compact)
                                } else {
                                    compact
                                }
                            }
                            _ => value.to_string(),
                        };
                        println!("{} {}", format!("{chunk_id}:").green(), &line);
                        inputs.push(chunk_id);
                        data.push(value);
                    }
                    Err(e) => {
                        print_transform_error(&expression, &e);
                        println!();
                        continue;
                    }
                }
                index += 1;
                println!();
            }

            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,

            Err(error) => {
                io::printerr!("Unexpected error:", error);
                println!();
                break;
            }
        }
    }

    let _ = readlines.save_history(&history_path);
}
