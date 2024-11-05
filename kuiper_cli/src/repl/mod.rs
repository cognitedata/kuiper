mod cmd_helper;
mod io;
mod magic;

use std::time::Instant;

use colored::Colorize;
use kuiper_lang::compile_expression;

use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

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

    let _ = readlines.load_history(&history_path);

    println!("Kuiper REPL version {}", env!("CARGO_PKG_VERSION"));
    println!("Type /help for a list of available commands.");

    loop {
        println!();
        let line = readlines.readline("kuiper> ");

        match line {
            Ok(expression) => {
                let _ = readlines.add_history_entry(expression.as_str());

                if expression.starts_with('/') {
                    match apply_magic_function(expression, &mut data, &mut inputs, &mut index) {
                        magic::ReplResult::Continue => continue,
                        magic::ReplResult::Stop => break,
                    }
                }

                let chunk_id = format!("var{index}");
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
                        io::printerr!("", e);
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
                        println!("{chunk_id} = {}", &*x);
                        inputs.push(chunk_id);
                        data.push(x.into_owned());
                    }
                    Err(e) => {
                        io::printerr!("Transform failed:", e);
                        continue;
                    }
                }
                index += 1;
            }

            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,

            Err(error) => {
                io::printerr!("Unexpected error:", error);
                break;
            }
        }
    }

    let _ = readlines.save_history(&history_path);
}
