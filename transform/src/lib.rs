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
    use serde_json::{json, Value};

    use crate::{Program, TransformInput};

    #[test]
    pub fn test_merge() {
        let raw: Vec<TransformInput> = serde_json::from_value(json!([
            {
                "id": "gen",
                "inputs": [],
                "transform": "[1, 2]",
                "type": "flatten"
            }, {
                "id": "parse",
                "inputs": ["input"],
                "transform": "$input.values",
                "type": "flatten"
            }, {
                "id": "finmerge",
                "inputs": ["gen", "parse"],
                "transform": {
                    "val": "$merge",
                },
                "type": "map",
                "mode": "merge"
            }
        ]))
        .unwrap();
        let program = Program::compile(raw).unwrap();
        let input = json!({
            "values": [3, 4, 5]
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 5);
        let mut vals: Vec<_> = res
            .into_iter()
            .map(|e| e.get("val").unwrap().as_u64().unwrap())
            .collect();
        // No guarantee of ordering
        vals.sort();
        for i in 0..5 {
            assert_eq!(*vals.get(i).unwrap(), (i + 1) as u64);
        }
    }

    #[test]
    pub fn test_zip() {
        let raw: Vec<TransformInput> = serde_json::from_value(json!([
            {
                "id": "gen",
                "inputs": [],
                "transform": "[1, 2]",
                "type": "flatten"
            }, {
                "id": "parse",
                "inputs": ["input"],
                "transform": "$input.values",
                "type": "flatten"
            }, {
                "id": "finmerge",
                "inputs": ["gen", "parse"],
                "transform": {
                    "gen": "$gen",
                    "parse": "$parse"
                },
                "type": "map",
                "mode": "zip"
            }
        ]))
        .unwrap();
        let program = Program::compile(raw).unwrap();
        let input = json!({
            "values": [3, 4, 5]
        });
        let res = program.execute(&input).unwrap();
        assert_eq!(res.len(), 3);
        let val = res.get(0).unwrap();
        assert_eq!(val.get("gen").unwrap(), 1);
        assert_eq!(val.get("parse").unwrap(), 3);
        let val = res.get(1).unwrap();
        assert_eq!(val.get("gen").unwrap(), 2);
        assert_eq!(val.get("parse").unwrap(), 4);
        let val = res.get(2).unwrap();
        assert_eq!(val.get("gen").unwrap(), &Value::Null);
        assert_eq!(val.get("parse").unwrap(), 5);
    }

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
