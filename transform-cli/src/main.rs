use std::{collections::HashMap, io};

use json_transform::compile_expression;

fn main() {
    let mut data = Vec::new();
    let mut index = 0usize;
    let mut inputs = HashMap::new();

    loop {
        println!("");
        println!("Input expression: ");
        let mut expr = String::new();
        io::stdin()
            .read_line(&mut expr)
            .expect("Unable to get user input");

        if &expr == "clear" {
            println!("Clearing stored inputs");
            index = 0;
            inputs.clear();
            data.clear();
            continue;
        } else if &expr == "exit" {
            break;
        }

        let chunk_id = format!("var{index}");
        let res = compile_expression(&expr, &mut inputs, &chunk_id);
        let expr = match res {
            Ok(x) => x,
            Err(e) => {
                println!("Compilation failed! {e}");
                continue;
            }
        };

        let res = expr.run(data.iter(), &chunk_id);
        match res {
            Ok(x) => {
                println!("{chunk_id} = {x}");
                inputs.insert(chunk_id, index);
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
