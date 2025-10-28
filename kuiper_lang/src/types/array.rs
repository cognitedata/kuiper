use std::fmt::Display;

use serde::Serialize;

use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
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
            return Some(end_dynamic.as_ref().clone().union_with(Type::null()));
        }
        None
    }

    pub fn all_elements(&self) -> impl Iterator<Item = &Type> {
        self.elements
            .iter()
            .chain(self.end_dynamic.iter().map(|d| d.as_ref()))
    }

    pub fn union_with(self, other: Array) -> Self {
        let mut res = Vec::new();
        let mut iter_1 = self.elements.into_iter().peekable();
        let mut iter_2 = other.elements.into_iter().peekable();

        // First, iterate through the known elements, unioning them pairwise,
        // so long as both arrays have known elements.
        while iter_1.peek().is_some() && iter_2.peek().is_some() {
            let Some(elem_1) = iter_1.next() else {
                break;
            };
            let Some(elem_2) = iter_2.next() else {
                break;
            };
            res.push(elem_1.union_with(elem_2));
        }

        let mut end_dynamic = None;
        if let Some(end_dynamic_1) = self.end_dynamic {
            if let Some(end_dynamic_2) = other.end_dynamic {
                // Both arrays have dynamic end, we need to union with anything
                // left in the iterators.
                let mut dynamic = end_dynamic_1.union_with(end_dynamic_2.as_ref().clone());
                for elem in iter_1.chain(iter_2) {
                    dynamic = dynamic.union_with(elem);
                }
                end_dynamic = Some(Box::new(dynamic));
            } else {
                // Only array 1 has dynamic end, so we can include anything left in it,
                // and merge the rest of array 2 into the dynamic end.
                let mut dynamic = *end_dynamic_1;
                for elem in iter_2 {
                    dynamic = dynamic.union_with(elem);
                }
                res.extend(iter_1);
                end_dynamic = Some(Box::new(dynamic));
            }
        } else if let Some(end_dynamic_2) = other.end_dynamic {
            // Only array 2 has dynamic end, so we can include anything left in it,
            // and merge the rest of array 1 into the dynamic end.
            let mut dynamic = *end_dynamic_2;
            for elem in iter_1 {
                dynamic = dynamic.union_with(elem);
            }
            res.extend(iter_2);
            end_dynamic = Some(Box::new(dynamic));
        } else {
            // Neither sequence has dynamic end, so we can just extend with the rest.
            // Note that one of these will be empty.
            res.extend(iter_1);
            res.extend(iter_2);
        }

        Array {
            elements: res,
            end_dynamic,
        }
    }

    pub fn from_const(value: Vec<serde_json::Value>) -> Self {
        let elements = value.into_iter().map(Type::from_const).collect::<Vec<_>>();
        Array {
            elements,
            end_dynamic: None,
        }
    }

    pub fn is_assignable_to(&self, other: &Array) -> bool {
        let mut self_iter = self.elements.iter();
        let mut other_iter = other.elements.iter();

        loop {
            let Some(self_elem) = self_iter.next() else {
                // If we run out of elements in self, all remaining elements in other must be
                // compatible with the end dynamic of self.
                if let Some(end_dynamic) = &self.end_dynamic {
                    for other_elem in other_iter {
                        if !end_dynamic.is_assignable_to(other_elem) {
                            return false;
                        }
                    }
                    return true;
                } else {
                    // If self has no end dynamic, and there are still elements in other,
                    // it means that they are not compatible.
                    return other_iter.next().is_none();
                }
            };
            let Some(other_elem) = other_iter.next() else {
                // If we run out of elements in other, all remaining elements in self must be
                // compatible with the end dynamic of other.
                if let Some(end_dynamic) = &other.end_dynamic {
                    if !self_elem.is_assignable_to(end_dynamic) {
                        return false;
                    }
                    for self_elem in self_iter {
                        if !self_elem.is_assignable_to(end_dynamic) {
                            return false;
                        }
                    }
                    return true;
                } else {
                    // If other has no end dynamic, it means it is not compatible with any remaining
                    // elements in self.
                    return false;
                }
            };

            if !self_elem.is_assignable_to(other_elem) {
                return false;
            }
        }
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
        assert_eq!(
            arr.index_into(2),
            Some(Type::Union(vec![Type::Float, Type::null()]))
        );
        assert_eq!(
            arr.index_into(100),
            Some(Type::Union(vec![Type::Float, Type::null()]))
        );
    }

    #[test]
    fn test_array_union() {
        // Shorter with dynamic end.
        let arr1 = Array {
            elements: vec![Type::String, Type::from_const(123)],
            end_dynamic: Some(Box::new(Type::Float)),
        };
        let arr2 = Array {
            elements: vec![
                Type::from_const("abc"),
                Type::from_const(456),
                Type::Boolean,
            ],
            end_dynamic: None,
        };
        let un = arr1.union_with(arr2);
        assert_eq!(
            un,
            Array {
                elements: vec![
                    Type::String,
                    Type::Union(vec![Type::from_const(123), Type::from_const(456)]),
                ],
                end_dynamic: Some(Type::Union(vec![Type::Float, Type::Boolean]).into()),
            }
        );

        // Longer with dynamic end.
        let arr1 = Array {
            elements: vec![Type::String, Type::from_const(123), Type::Boolean],
            end_dynamic: Some(Box::new(Type::Float)),
        };
        let arr2 = Array {
            elements: vec![Type::from_const("abc"), Type::from_const(456)],
            end_dynamic: None,
        };
        let un = arr1.union_with(arr2);
        assert_eq!(
            un,
            Array {
                elements: vec![
                    Type::String,
                    Type::Union(vec![Type::from_const(123), Type::from_const(456)]),
                    Type::Boolean,
                ],
                end_dynamic: Some(Type::Float.into()),
            }
        );

        // Neither has dynamic end.
        let arr1 = Array {
            elements: vec![Type::String, Type::from_const(123)],
            end_dynamic: None,
        };
        let arr2 = Array {
            elements: vec![
                Type::from_const("abc"),
                Type::from_const(456),
                Type::Boolean,
            ],
            end_dynamic: None,
        };
        let un = arr1.union_with(arr2);
        assert_eq!(
            un,
            Array {
                elements: vec![
                    Type::String,
                    Type::Union(vec![Type::from_const(123), Type::from_const(456)]),
                    Type::Boolean,
                ],
                end_dynamic: None,
            }
        );
    }
}
