use clap::{Parser, ValueEnum};
use kuiper_cli::errors::KuiperCliError;
use kuiper_cli::repl::repl;
use kuiper_lang::compile_expression;
use serde_json::Value;
use std::fs::{self, read_to_string};
use std::io;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum MessageEnd {
    Eof,
    LF,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Kuiper expression to run
    #[arg(short, long)]
    expression: Option<String>,

    /// File to load kuiper expression to run from
    #[arg(short = 'f', long)]
    expression_file: Option<PathBuf>,

    /// Input data, uses STDIN if omitted
    input: Option<PathBuf>,

    /// Message separator
    #[arg(short, long, value_enum, default_value = "eof")]
    separator: MessageEnd,

    /// Verbose logging
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Run formatter on the expression. If expression file is provided, it will be overwritten with the formatted expression unless output file is specified.
    #[arg(long)]
    format: bool,

    /// Output results to a file instead of STDOUT
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl Args {
    pub fn launch_repl(&self) -> bool {
        self.expression.is_none() && self.expression_file.is_none() && self.input.is_none()
    }
}

fn load_input_data(args: &Args) -> Result<Vec<Value>, KuiperCliError> {
    let string_data = match &args.input {
        Some(path) => read_to_string(path)?,
        None => {
            let mut buffer = Vec::new();
            io::stdin().read_to_end(&mut buffer)?;
            String::from_utf8(buffer)?
        }
    };

    let data = match &args.separator {
        MessageEnd::LF => string_data
            .trim()
            .split('\n')
            .map(serde_json::from_str::<Value>)
            .collect::<Result<Vec<Value>, serde_json::Error>>()?,
        MessageEnd::Eof => vec![serde_json::from_str(&string_data)?],
    };

    Ok(data)
}

fn load_expression(args: &Args) -> Result<String, KuiperCliError> {
    match (&args.expression, &args.expression_file) {
        (None, None) => Err("Either expression or expression file needs to be provided!")?,
        (Some(expression), None) => Ok(expression.clone()),
        (None, Some(file)) => Ok(read_to_string(file)?),
        _ => Err("Only expression or expression file can be provided!")?,
    }
}

fn run_formatter(args: &Args) -> Result<String, KuiperCliError> {
    let expression = load_expression(args)?;
    let formatted_expression = kuiper_lang::format_expression(&expression)?;
    Ok(formatted_expression)
}

fn inner_run(args: &Args) -> Result<Vec<String>, KuiperCliError> {
    let expression = load_expression(&args)?;

    let expression = compile_expression(&expression, &["input"])?;

    let data = load_input_data(&args)?;

    let mut res = Vec::new();
    for input in data {
        let (result, opcount) = expression.run_get_opcount([&input])?;
        res.push(serde_json::to_string(&*result)?);

        if args.verbose {
            println!("Expression executed with {opcount} operations");
        }
    }

    Ok(res)
}

pub fn main() {
    let args = Args::parse();

    if args.launch_repl() {
        repl(args.verbose);
        return;
    }

    match args.format {
        true => match run_formatter(&args) {
            Ok(formatted) => {
                if let Some(path) = &args.output {
                    if let Err(e) = fs::write(path, formatted) {
                        eprintln!(
                            "\x1b[91mError writing to file {}:\x1b[0m {e}",
                            path.display()
                        );
                    }
                } else if let Some(path) = &args.expression_file {
                    if let Err(e) = fs::write(path, formatted) {
                        eprintln!(
                            "\x1b[91mError writing to file {}:\x1b[0m {e}",
                            path.display()
                        );
                    }
                } else {
                    println!("{formatted}");
                }
            }
            Err(error) => eprintln!("\x1b[91mError:\x1b[0m {error}"),
        },

        false => match inner_run(&args) {
            Ok(strings) => strings.into_iter().for_each(|s| {
                if let Some(path) = &args.output {
                    fs::write(path, s).unwrap_or_else(|e| {
                        eprintln!(
                            "\x1b[91mError writing to file {}:\x1b[0m {e}",
                            path.display()
                        )
                    });
                } else {
                    println!("{s}");
                }
            }),
            Err(error) => eprintln!("\x1b[91mError:\x1b[0m {error}"),
        },
    }
}
