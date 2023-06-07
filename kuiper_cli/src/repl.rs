use std::io;

use kuiper_lang::compile_expression;

pub fn repl() {
    let mut data = Vec::new();
    let mut index = 0usize;
    let mut inputs = Vec::<String>::new();

    loop {
        println!();
        println!("Input expression: ");
        let mut expr = String::new();
        io::stdin()
            .read_line(&mut expr)
            .expect("Unable to get user input");

        if expr.trim_end().eq("clear") {
            println!("Clearing stored inputs");
            index = 0;
            inputs.clear();
            data.clear();
            continue;
        } else if expr.trim_end().eq("exit") {
            break;
        }

        let chunk_id = format!("var{index}");
        let res = compile_expression(
            &expr,
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
    }
}
