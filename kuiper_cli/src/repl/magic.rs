use std::vec;

use colored::Colorize;
use serde_json::Value;

use crate::builtins::{BUILT_INS, HELP};

use super::io::printerr;

fn help(command: Option<&str>) {
    match command {
        Some(command_name) => match HELP.get(command_name) {
            Some(function) => {
                println!(
                    "{}",
                    format!("Help page for {}", command_name).bold().underline()
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
                ("/clear", "Clear all stored values"),
                (
                    "/help (<function>)",
                    "Display this help page or help for specific functions",
                ),
                ("/store <name>", "Store the last result as a named variable"),
                ("/exit", "Quit the REPL"),
            ]
            .into_iter()
            .for_each(|(func, desc)| println!("  {:-25}{}", func, desc));
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
) -> ReplResult {
    let parsed_line: Vec<&str> = line.split_whitespace().collect();

    match parsed_line.first() {
        Some(&"/help") => {
            help(parsed_line.get(1).map(|s| s.to_owned()));
            ReplResult::Continue
        }

        Some(&"/clear") => {
            println!("Clearing stored values");
            *index = 0;
            inputs.clear();
            data.clear();

            ReplResult::Continue
        }

        Some(&"/store") => {
            match parsed_line.get(1) {
                Some(name) => match (inputs.last(), data.last()) {
                    (Some(old_name), Some(value)) => {
                        println!("Storing {} as {}", old_name, name);
                        inputs.append(&mut vec![name.to_string()]);
                        data.append(&mut vec![value.clone()]);
                    }
                    _ => printerr!("No data to store", ""),
                },
                None => printerr!("Missing name of variable to store value into", ""),
            };

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
