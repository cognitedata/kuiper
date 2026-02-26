use std::{collections::BTreeMap, fmt::Display};

use logos::Span;
use serde_json::{Map, Value};

use crate::{
    compiler::BuildError,
    types::{Object, ObjectField, Type},
    write_list, TransformError,
};

use super::{base::ExpressionMeta, Expression, ExpressionType, ResolveResult};

#[derive(Debug)]
pub enum ObjectElement {
    Pair(ExpressionType, ExpressionType),
    Concat(ExpressionType),
}

impl Display for ObjectElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(key, value) => write!(f, "{key}: {value}"),
            Self::Concat(x) => write!(f, "...{x}"),
        }
    }
}

#[derive(Debug)]
pub struct ObjectExpression {
    items: Vec<ObjectElement>,
    span: Span,
}

impl Display for ObjectExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        write_list!(f, &self.items);
        write!(f, "}}")?;
        Ok(())
    }
}

impl Expression for ObjectExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'a, '_>,
    ) -> Result<super::ResolveResult<'a>, crate::TransformError> {
        state.inc_op()?;
        let mut output = Map::with_capacity(self.items.len());
        for k in self.items.iter() {
            match k {
                ObjectElement::Pair(key, value) => {
                    let key_res = key.resolve(state)?;
                    let key_val = key_res.try_into_string("object", &self.span)?;
                    output.insert(key_val.into_owned(), value.resolve(state)?.into_owned());
                }
                ObjectElement::Concat(x) => {
                    let conc = x.resolve(state)?;
                    match conc {
                        ResolveResult::Owned(Value::Object(x)) => {
                            for (k, v) in x {
                                output.insert(k, v);
                            }
                        }
                        ResolveResult::Borrowed(Value::Object(x)) => {
                            for (k, v) in x {
                                output.insert(k.to_owned(), v.to_owned());
                            }
                        }
                        x => {
                            return Err(TransformError::new_incorrect_type(
                                "object",
                                "object",
                                TransformError::value_desc(&x),
                                &self.span,
                            ))
                        }
                    };
                }
            }
        }
        Ok(ResolveResult::Owned(Value::Object(output)))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        let mut output = BTreeMap::new();
        for k in self.items.iter() {
            match k {
                ObjectElement::Pair(key, value) => {
                    let key_type = key.resolve_types(state)?;
                    let value_type = value.resolve_types(state)?;
                    key_type.assert_assignable_to(
                        &Type::String
                            .union_with(Type::number())
                            .union_with(Type::Boolean)
                            .union_with(Type::null()),
                        &self.span,
                    )?;
                    if let Type::Constant(Value::String(key_str)) = key_type {
                        output.insert(ObjectField::Constant(key_str), value_type);
                    } else if let Some(old) = output.remove(&ObjectField::Generic) {
                        output.insert(ObjectField::Generic, old.union_with(value_type));
                    } else {
                        output.insert(ObjectField::Generic, value_type);
                    }
                }
                ObjectElement::Concat(x) => {
                    let conc_type = x.resolve_types(state)?;
                    let conc_obj = conc_type.try_as_object(&self.span)?;
                    for (k, v) in conc_obj.fields {
                        match k {
                            ObjectField::Constant(key_str) => {
                                output.insert(ObjectField::Constant(key_str), v);
                            }
                            ObjectField::Generic => {
                                if let Some(old) = output.remove(&ObjectField::Generic) {
                                    output.insert(ObjectField::Generic, old.union_with(v));
                                } else {
                                    output.insert(ObjectField::Generic, v);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Type::Object(Object { fields: output }))
    }
}

impl ExpressionMeta for ObjectExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.items.iter_mut().flat_map(|f| match f {
            ObjectElement::Pair(x, y) => vec![x, y].into_iter(),
            ObjectElement::Concat(x) => vec![x].into_iter(),
        }))
    }
}

impl ObjectExpression {
    pub fn new(items: Vec<ObjectElement>, span: Span) -> Result<Self, BuildError> {
        for k in &items {
            match k {
                ObjectElement::Pair(key, val) => {
                    key.fail_if_lambda()?;
                    val.fail_if_lambda()?;
                }
                ObjectElement::Concat(x) => {
                    x.fail_if_lambda()?;
                }
            }
        }
        Ok(Self { items, span })
    }
}

#[cfg(test)]
mod tests {
    use crate::types::Type;

    #[test]
    fn test_object_types() {
        let expr = crate::compile_expression(
            r#"
        {
            "a": 5,
            "b": "hello",
            ...{"b": 6, "c": true, "d": null},
            ...{"e": input.value}
        }
        "#,
            &["input"],
        )
        .unwrap();
        let r = expr.run_types([Type::Any]).unwrap();
        assert_eq!(
            r,
            Type::Object(
                crate::types::Object::default()
                    .with_field("a", Type::from_const(5))
                    .with_field("b", Type::from_const(6))
                    .with_field("c", Type::from_const(true))
                    .with_field("d", Type::null())
                    .with_field("e", Type::Any)
            )
        );
    }

    #[test]
    fn test_object_wrong_key_type() {
        let expr = crate::compile_expression(
            r#"
            {
                input: "value"
            }
            "#,
            &["input"],
        )
        .unwrap();

        let r = expr
            .run_types([Type::object_of_type(Type::String)])
            .unwrap_err();
        assert_eq!(
            r.to_string(),
            "Expected Union<Integer, Float, String, Boolean, null> but got {...: String}"
        );
    }
}
