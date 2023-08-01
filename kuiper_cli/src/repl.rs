use kuiper_lang::compile_expression;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Context, Editor, Helper};
use rustyline::{Highlighter, Hinter, Validator};

#[derive(Hinter, Highlighter, Validator, Helper)]
struct KuiperHelper {}

impl KuiperHelper {
    pub fn new() -> Self {
        KuiperHelper {}
    }
}

fn is_separator(c: Option<char>) -> bool {
    match c {
        None | Some(',') | Some(' ') | Some(':') | Some('\n') | Some(')') | Some('(')
        | Some('"') => true,
        Some(_) => false,
    }
}

impl Completer for KuiperHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (mut low, mut high) = (pos.saturating_sub(1), pos);

        while !is_separator(line.chars().nth(low)) && low > 0 {
            low -= 1;
        }
        if low != 0 {
            low += 1;
        }
        while !is_separator(line.chars().nth(high)) {
            high += 1;
        }

        let word: String = line.chars().skip(low).take(high - low).collect();
        let candidates: Vec<String> = vec![
            "pow(",
            "log(",
            "atan2(",
            "floor(",
            "ceil(",
            "round(",
            "concat(",
            "string(",
            "int(",
            "float(",
            "try_float(",
            "try_int(",
            "try_bool(",
            "if(",
            "to_unix_timestamp(",
            "format_timestamp(",
            "case(",
            "pairs(",
            "map(",
            "flatmap(",
            "reduce(",
            "filter(",
            "zip(",
            "length(",
            "chunk(",
            "now(",
            "input",
        ]
        .into_iter()
        .filter(|s| s.starts_with(&word))
        .map(|s| s.to_string())
        .collect();

        Ok((low, candidates))
    }
}

pub fn repl() {
    let mut data = Vec::new();
    let mut index = 0usize;
    let mut inputs = Vec::<String>::new();

    let editor_config = Config::builder()
        .completion_type(CompletionType::List)
        .build();

    let mut readlines = Editor::with_config(editor_config).unwrap();
    readlines.set_helper(Some(KuiperHelper::new()));

    let mut history_path = dirs::home_dir().unwrap();
    history_path.push(".kuiper_history");

    let _ = readlines.load_history(&history_path);

    loop {
        let line = readlines.readline("kuiper> ");

        match line {
            Ok(expression) => {
                let _ = readlines.add_history_entry(expression.as_str());
                if expression.trim_end().eq("clear") {
                    println!("Clearing stored inputs");
                    index = 0;
                    inputs.clear();
                    data.clear();
                    continue;
                } else if expression.trim_end().eq("exit") {
                    break;
                }

                let chunk_id = format!("var{index}");
                let res = compile_expression(
                    &expression,
                    &inputs.iter().map(String::as_str).collect::<Vec<_>>(),
                );
                let expr = match res {
                    Ok(x) => x,
                    Err(e) => {
                        println!("Compilation failed! {e}");
                        continue;
                    }
                };

                let res = expr.run(data.iter());
                match res {
                    Ok(x) => {
                        println!("{chunk_id} = {x}");
                        inputs.push(chunk_id);
                        data.push(x.into_owned());
                    }
                    Err(e) => {
                        println!("Transform failed! {e}");
                        continue;
                    }
                }
                index += 1;
                println!();
            }

            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,

            Err(error) => {
                eprintln!("Unexpected error: {}", error);
                break;
            }
        }

        let _ = readlines.save_history(&history_path);
    }
}
