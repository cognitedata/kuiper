use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::expressions::{
    numbers::JsonNumber, Expression, ExpressionExecutionState, ResolveResult,
};

function_def!(DigestFunction, "digest", 1, None);

fn hash_value_rec(value: &Value, hasher: &mut Sha256) {
    match value {
        Value::Null => hasher.update([0u8]),
        Value::Bool(b) => hasher.update(if *b { [1u8] } else { [2u8] }),
        Value::Number(n) => {
            hasher.update([4u8]);
            hasher.update(match JsonNumber::from(n) {
                JsonNumber::NegInteger(v) => v.to_be_bytes(),
                JsonNumber::PosInteger(v) => v.to_be_bytes(),
                JsonNumber::Float(v) => v.to_be_bytes(),
            })
        }
        Value::String(s) => {
            hasher.update([8u8]);
            hasher.update(s)
        }
        Value::Array(a) => {
            hasher.update([16u8]);
            hasher.update(a.len().to_be_bytes());
            for v in a {
                hash_value_rec(v, hasher);
            }
        }
        Value::Object(o) => {
            hasher.update([32u8]);
            hasher.update(o.len().to_be_bytes());
            for (k, v) in o {
                hasher.update(k);
                hash_value_rec(v, hasher);
            }
        }
    }
}

impl<'a: 'c, 'c> Expression<'a, 'c> for DigestFunction {
    fn resolve(
        &'a self,
        state: &mut ExpressionExecutionState<'c, '_>,
    ) -> Result<ResolveResult<'c>, crate::TransformError> {
        let mut hasher = sha2::Sha256::new();
        for arg in &self.args {
            hash_value_rec(arg.resolve(state)?.as_ref(), &mut hasher);
        }

        let val = hasher.finalize();
        let base64_out = base64::engine::general_purpose::STANDARD.encode(val);
        Ok(ResolveResult::Owned(Value::String(base64_out)))
    }

    fn resolve_types(
        &'a self,
        state: &mut crate::expressions::types::TypeExecutionState<'c, '_>,
    ) -> Result<crate::expressions::types::Type, crate::expressions::types::TypeError> {
        for arg in &self.args {
            arg.resolve_types(state)?;
        }
        Ok(crate::expressions::types::Type::String)
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use crate::compile_expression;

    #[test]
    fn test_digest() {
        let expr = compile_expression(
            r#"
        digest("test", 123, 321.321, [1, 2, 3], { "a": "b", "c": "d" })
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let val = res.as_str().unwrap();
        assert_eq!("iVGAE6wehaUtbh2VF98pAlI1akTiRxB88dflW9xUGaM=", val);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(val)
            .unwrap();
        // This is SHA256, so decoded it should be 256 bits.
        assert_eq!(decoded.len(), 256 / 8);
    }

    #[test]
    fn test_digest_eq() {
        let expr = compile_expression(
            r#"
            digest("test", "foo", 123) == digest("test", "foo", 123)
        "#,
            &[],
        )
        .unwrap();
        let res = expr.run([]).unwrap();
        let val = res.as_bool();
        assert!(val);
    }
}
