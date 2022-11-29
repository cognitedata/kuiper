use program::{Program, TransformInput};
use serde_json::json;

mod expressions;
mod lexer;
mod parse;
mod program;

fn main() {
    let raw: Vec<TransformInput> = serde_json::from_value(json!([
        {
            "id": "step1",
            "inputs": ["input"],
            "transform": "$input.values",
            "type": "flatten"
        },
        {
            "id": "unused",
            "inputs": ["input"],
            "transform": {
                "some-unused-transform": "This can contain syntax errors",
                "since": "it won't be compiled"
            },
            "type": "map"
        },
        {
            "id": "step2",
            "inputs": ["input", "step1"],
            "transform": {
                "externalId": "$input.id",
                "value": "$step1.value * pow(10, $step1.valueExponent)",
                "timestamp": "$step1.time"
            },
            "type": "map"
        }
    ]))
    .unwrap();

    let program = Program::compile(raw).unwrap();
    let input = json!({
        "id": "my-id",
        "values": [{
            "value": 123.123,
            "valueExponent": 5,
            "time": 123142812824u64
        }, {
            "value": 321.321,
            "valueExponent": 5,
            "time": 123901591231u64
        }]
    });
    let res = program.execute(input).unwrap();
    for rs in res {
        println!("{}", rs);
    }

    /*let mut input = HashMap::new();
    let inp = json!({
        "elem": 3,
        "elem2": 4,
        "arr": [1, 2, 3],
        "deep": [{
            "nested": 10
        }]
    });
    input.insert("id".to_string(), &inp);

    // Fancy array selectors
    let lex = Token::lexer("1 + $id.elem + 2 + 3 + floor(2.5)");
    let res = Parser::new(lex).parse().unwrap();
    let state = ExpressionExecutionState { data: input };
    println!("{}", res.resolve(&state).unwrap().as_ref());
    println!("{}", res);

    // let lex = Token::lexer("[0, 1, 2, 3, $id.arr]");
    let lex = Token::lexer("$id.deep[0].nested");
    let res = Parser::new(lex).parse().unwrap();
    println!("{}", res.resolve(&state).unwrap().as_ref());

    println!("{}", res); */
}
