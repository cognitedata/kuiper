mod expressions;
mod lexer;
mod parse;
mod program;

pub use program::CompileError;
pub use program::{Program, TransformInput};

pub use parse::{Parser, ParserError, ParserErrorData};

pub use expressions::{TransformError, TransformErrorData};

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{Program, TransformInput};

    #[test]
    pub fn test_exponential_flatten() {
        let raw: Vec<TransformInput> = serde_json::from_value(json!([
            {
                "id": "step1",
                "inputs": ["input"],
                "transform": "$input.values",
                "type": "flatten"
            },
            {
                "id": "gen",
                "inputs": [],
                "transform": "[0, 1, 2, 3, 4]",
                "type": "flatten"
            },
            {
                "id": "explode1",
                "inputs": ["gen", "step1"],
                "transform": {
                    "v1": "$gen",
                    "v2": "$step1.value"
                },
                "type": "map"
            },
            {
                "id": "explode2",
                "inputs": ["gen", "explode1"],
                "transform": {
                    "v1": "$gen",
                    "v21": "$explode1.v1",
                    "v22": "$explode1.v2"
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
                "time": 123142812824u64
            }, {
                "value": 321.321,
                "time": 123901591231u64
            }]
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 50);
        for rs in res {
            println!("{}", rs);
        }
    }
}
