use kuiper_lang::compile_expression;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub fn repl() {
    let mut data = Vec::new();
    let mut index = 0usize;
    let mut inputs = Vec::<String>::new();

    let mut readlines = DefaultEditor::new().unwrap();
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
