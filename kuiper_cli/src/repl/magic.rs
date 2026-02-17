use std::{cmp::max, collections::HashMap, vec};

use colored::Colorize;
use kuiper_lang::types::Type;
use serde_json::Value;

use crate::builtins::{BUILT_INS, HELP};

use super::{io::printerr, macros::Macro};

fn help(command: Option<&str>) {
    match command {
        Some(command_name) => match HELP.get(command_name) {
            Some(function) => {
                println!(
                    "{}",
                    format!("Help page for {command_name}").bold().underline()
                );
                println!("Signature:  {}\n", function.signature);
                println!("{}", function.description);
            }

            None => printerr!(format!("No function named {command_name}"), ""),
        },

        None => {
            println!("Type a Kuiper expression to evaluate it.\n");
            println!("{}", "Built-in functions".bold().underline());
            BUILT_INS
                .iter()
                .map(|f| f.trim_end_matches('('))
                .for_each(|f| {
                    println!(
                        "  {:-25}{}",
                        f,
                        HELP.get(f)
                            .map(|fd| fd.description)
                            .map(|d| d.split('.').collect::<Vec<&str>>()[0])
                            .unwrap_or_default()
                    )
                });
            println!("\nType /help <command> to get more info about a specific command\n");
            println!(
                "The Kuiper REPL also supports {} which are meta-functions to the REPL itself\n",
                "magic functions".italic()
            );
            println!("{}", "Magic functions".bold().underline());
            [
                ("/clear", "Clear all stored values and macros"),
                (
                    "/help (<function>)",
                    "Display this help page or help for specific functions",
                ),
                ("/store <name>", "Store the last result as a named variable"),
                ("/macros", "List all stored macros and their definitions"),
                (
                    "/type <expression>",
                    "Determine the resulting type of an expression",
                ),
                ("/exit", "Quit the REPL"),
            ]
            .into_iter()
            .for_each(|(func, desc)| println!("  {func:-25}{desc}"));
        }
    }
}

pub enum ReplResult {
    Continue,
    Stop,
}

pub fn apply_magic_function(
    line: String,
    data: &mut Vec<Value>,
    inputs: &mut Vec<String>,
    index: &mut usize,
    macro_defs: &mut HashMap<String, Macro>,
) -> ReplResult {
    let parsed_line: Vec<&str> = line.split_whitespace().collect();

    match parsed_line.first() {
        Some(&"/help") => {
            help(parsed_line.get(1).map(|s| s.to_owned()));
            ReplResult::Continue
        }

        Some(&"/clear") => {
            println!("Clearing stored values and macros");
            *index = 0;
            inputs.clear();
            data.clear();
            macro_defs.clear();

            ReplResult::Continue
        }

        Some(&"/store") => {
            match parsed_line.get(1) {
                Some(name) => match (inputs.last(), data.last()) {
                    (Some(old_name), Some(value)) => {
                        println!("Storing {old_name} as {name}");
                        inputs.append(&mut vec![name.to_string()]);
                        data.append(&mut vec![value.clone()]);
                    }
                    _ => printerr!("No data to store", ""),
                },
                None => printerr!("Missing name of variable to store value into", ""),
            };

            ReplResult::Continue
        }

        Some(&"/macros") => {
            if macro_defs.is_empty() {
                println!("No macros stored");
                ReplResult::Continue
            } else {
                let first_col_width = macro_defs
                    .keys()
                    .fold(0, |max_width, row| max(max_width, row.len()))
                    + 3;
                println!(
                    "{:-width$}{}",
                    "Name".bold(),
                    "Expression".bold(),
                    width = first_col_width
                );

                for (name, mac) in macro_defs {
                    println!("{:-width$}{}", name, mac.def, width = first_col_width)
                }

                ReplResult::Continue
            }
        }

        Some(&"/type") => {
            let raw_expression = line.trim_start_matches("/type").trim();

            if raw_expression.is_empty() {
                printerr!("Missing expression to determine type of", "");
                return ReplResult::Continue;
            }

            let expression = match kuiper_lang::compile_expression(
                raw_expression,
                &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
            ) {
                Ok(expr) => expr,
                Err(e) => {
                    printerr!(format!("{e}"), "");
                    return ReplResult::Continue;
                }
            };

            match expression.run_types((0..data.len()).map(|_| Type::Any)) {
                Ok(ty) => println!("{}", ty),
                Err(e) => printerr!(format!("Error determining type: {e}"), ""),
            }

            ReplResult::Continue
        }

        Some(&"/exit") => ReplResult::Stop,

        Some(other) => {
            printerr!(format!("No magic function named {}", other), "");
            ReplResult::Continue
        }
        None => {
            printerr!("Internal error", "Could not match a magic function");
            ReplResult::Continue
        }
    }
}
