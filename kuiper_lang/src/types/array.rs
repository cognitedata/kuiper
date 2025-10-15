use std::fmt::Display;

use serde::Serialize;

use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// A JSON array type, containing a sequence of types for known elements,
/// and an optional type for any additional elements found at the end.
pub struct Array {
    /// Known elements at the start of the array.
    pub elements: Vec<Type>,
    /// An optional type for any additional elements at the end of the array.
    pub end_dynamic: Option<Box<Type>>,
}

impl Array {
    /// Index into the array from the start, returning the type of the element if it could exist.
    pub fn index_into(&self, index: usize) -> Option<Type> {
        if let Some(elem) = self.elements.get(index) {
            return Some(elem.clone());
        }
        if let Some(end_dynamic) = &self.end_dynamic {
            // TODO: Union with null here once we have implemented union calculation.
            return Some(end_dynamic.as_ref().clone());
        }
        None
    }

    pub fn all_elements(&self) -> impl Iterator<Item = &Type> {
        self.elements
            .iter()
            .chain(self.end_dynamic.iter().map(|d| d.as_ref()))
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut needs_comma = false;
        for elem in &self.elements {
            if needs_comma {
                write!(f, ", ")?;
            } else {
                needs_comma = true;
            }
            write!(f, "{}", elem)?;
        }
        if let Some(end_dynamic) = &self.end_dynamic {
            if needs_comma {
                write!(f, ", ")?;
            }
            write!(f, "...{}", end_dynamic)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Type;

    #[test]
    fn test_array_indexing() {
        let arr = Array {
            elements: vec![Type::String, Type::from_const(123)],
            end_dynamic: Some(Box::new(Type::Float)),
        };
        assert_eq!(arr.index_into(0), Some(Type::String));
        assert_eq!(arr.index_into(1), Some(Type::from_const(123)));
        assert_eq!(arr.index_into(2), Some(Type::Float));
        assert_eq!(arr.index_into(3), Some(Type::Float));
        assert_eq!(arr.index_into(100), Some(Type::Float));
    }
}
