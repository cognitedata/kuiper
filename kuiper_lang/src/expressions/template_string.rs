use std::fmt::Display;

use logos::Span;

use crate::{Expression, ExpressionMeta, ExpressionType};

#[derive(Debug)]
pub enum TemplateStringSegment {
    Raw(String),
    Expression(ExpressionType),
}

#[derive(Debug)]
pub struct TemplateStringExpression {
    pub segments: Vec<TemplateStringSegment>,
    pub span: Span,
}

impl Display for TemplateStringExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "$\"")?;
        for seg in &self.segments {
            match seg {
                TemplateStringSegment::Raw(s) => write!(f, "{s}")?,
                TemplateStringSegment::Expression(e) => write!(f, "{{{e}}}")?,
            }
        }
        write!(f, "\"")
    }
}

impl Expression for TemplateStringExpression {
    fn resolve<'a>(
        &'a self,
        state: &mut super::ExpressionExecutionState<'a, '_>,
    ) -> Result<super::ResolveResult<'a>, super::TransformError> {
        let mut result = String::new();
        for seg in &self.segments {
            match seg {
                TemplateStringSegment::Raw(s) => result.push_str(s),
                TemplateStringSegment::Expression(e) => {
                    let res = e.resolve(state)?;
                    result.push_str(&res.try_as_string("template string", &self.span)?);
                }
            }
        }
        Ok(super::ResolveResult::Owned(serde_json::Value::String(
            result,
        )))
    }

    fn resolve_types(
        &self,
        state: &mut crate::types::TypeExecutionState<'_, '_>,
    ) -> Result<crate::types::Type, crate::types::TypeError> {
        for expr in &self.segments {
            if let TemplateStringSegment::Expression(e) = expr {
                e.resolve_types(state)?;
            }
        }

        Ok(crate::types::Type::String)
    }
}

impl ExpressionMeta for TemplateStringExpression {
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ExpressionType> + '_> {
        Box::new(self.segments.iter_mut().filter_map(|m| match m {
            TemplateStringSegment::Raw(_) => None,
            TemplateStringSegment::Expression(expression_type) => Some(expression_type),
        }))
    }
}

impl TemplateStringExpression {
    pub fn new(segments: Vec<TemplateStringSegment>, span: Span) -> Self {
        Self { segments, span }
    }
}

#[cfg(test)]
mod tests {
    use crate::{compile_expression, BuildError};

    #[test]
    fn test_template_string() {
        let expr = compile_expression(
            r#"
        {
            // No expression segments
            "v1": $"Test1",
            // Single expression segment
            "v2": $"Test2 {concat('A', 'B')}",
            // Two adjecent segments
            "v3": $"Test3 {concat('A', 'B')}{concat('C', 'D')}",
            // Segment with braces
            "v4": $"Test4 {{{concat('A', 'B')}}}",
            // Segment with text between
            "v5": $"Test5 {'a'}+{'b'}",
            // Lambda inside template string
            "v6": $"Test6 {[1, 2, 3].map(x => x * 2).sum()}",
            // Template string inside another template string
            "v7": $"Test7 {concat('A', $'Test{1 + 2}C')}",
            // Template strings in expressions
            "v8": concat($"Test{1 + 2}", $"AB {concat('C', 'D')}"),
            // Empty template string
            "v9": $"",
            // Template string with only expression segments
            "v10": $"{concat('A', 'B')}{concat('C', 'D')}",
            // Comment inside template string
            "v11": $"Test11 {/* This is a comment and should be ignored */ 'F'} {concat('A', 'B')}",
            // Line comment inside template string is a little weird, but not so weird that I'm going to fix it..
            "v12": $"Test12 {'A' // This is a line comment and due to the way we did this it actually ends at the end of the template} B",
            // Multiline template string. All strings in kuiper are multiline, but newlines inside templates are ignored.
            "v13": $"Test13
{concat('A', 'B')}
{
    concat('C', 'D')
}"
        }
        "#,
            &[],
        )
        .unwrap();

        let res = expr.run([]).unwrap();
        assert_eq!(res["v1"].as_str().unwrap(), "Test1");
        assert_eq!(res["v2"].as_str().unwrap(), "Test2 AB");
        assert_eq!(res["v3"].as_str().unwrap(), "Test3 ABCD");
        assert_eq!(res["v4"].as_str().unwrap(), "Test4 {AB}");
        assert_eq!(res["v5"].as_str().unwrap(), "Test5 a+b");
        assert_eq!(res["v6"].as_str().unwrap(), "Test6 12");
        assert_eq!(res["v7"].as_str().unwrap(), "Test7 ATest3C");
        assert_eq!(res["v8"].as_str().unwrap(), "Test3AB CD");
        assert_eq!(res["v9"].as_str().unwrap(), "");
        assert_eq!(res["v10"].as_str().unwrap(), "ABCD");
        assert_eq!(res["v11"].as_str().unwrap(), "Test11 F AB");
        assert_eq!(res["v12"].as_str().unwrap(), "Test12 A B");
        assert_eq!(res["v13"].as_str().unwrap(), "Test13\nAB\nCD");
    }

    #[test]
    fn test_template_string_error_offset() {
        let expr = compile_expression(r#"$"test {bad_func(123)}""#, &[]).unwrap_err();
        match expr {
            crate::CompileError::Build(BuildError::UnrecognizedFunction(v)) => {
                assert_eq!(v.position.start, 8);
                assert_eq!(v.position.end, 21);
            }
            r => panic!("Expected build error got {r:?}"),
        }
    }
}
