#[macro_export]
macro_rules! function_def {
    // Base, should have defined the struct
    (_display $typ:ident) => {
        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(", <Self as $crate::expressions::functions::FunctionExpression<'_>>::INFO.name)?;
                let mut is_first = true;
                for expr in $crate::expressions::functions::FunctionExpression::<'_>::get_args(self) {
                    if !is_first {
                        write!(f, ", ")?;
                    }
                    is_first = false;
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
        }
    };
    ($typ:ident, $name:expr, $nargs:expr) => {
        #[derive(Debug)]
        pub struct $typ {
            args: [Box<$crate::expressions::base::ExpressionType>; $nargs],
            #[allow(dead_code)]
            span: logos::Span,
        }

        function_def!(_display $typ);

        impl<'a> $crate::expressions::functions::FunctionExpression<'a> for $typ {
            type Iter<'b> = std::iter::Map<std::slice::Iter<'b, Box<$crate::expressions::base::ExpressionType>>, fn(&'b Box<$crate::expressions::base::ExpressionType>) -> &'b $crate::expressions::base::ExpressionType>;// std::iter::Map<std::slice::Iter<'_, &ExpressionType>>;
            const INFO: $crate::expressions::functions::FunctionInfo = $crate::expressions::functions::FunctionInfo {
                minargs: $nargs,
                maxargs: Some($nargs),
                name: $name
            };

            fn new(args: Vec<$crate::expressions::base::ExpressionType>, span: logos::Span) -> Result<Self, $crate::parse::ParserError> {
                if !Self::INFO.validate(args.len()) {
                    return Err($crate::ParserError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                Ok(Self {
                    span,
                    args: args.into_iter().map(|a| Box::new(a)).collect::<Vec<_>>().try_into().unwrap()
                })
            }

            fn get_args(&'a self) -> Self::Iter<'a> {
                self.args.iter().map(|a| &a)
            }
        }
    };
    ($typ:ident, $name:expr, $minargs:expr, $maxargs:expr) => {
        #[derive(Debug)]
        pub struct $typ {
            args: Vec<$crate::expressions::base::ExpressionType>,
            #[allow(dead_code)]
            span: logos::Span
        }

        function_def!(_display $typ);

        impl<'a> $crate::expressions::functions::FunctionExpression<'a> for $typ {
            type Iter<'b> = std::slice::Iter<'b, $crate::expressions::base::ExpressionType>;
            const INFO: $crate::expressions::functions::FunctionInfo = $crate::expressions::functions::FunctionInfo {
                minargs: $minargs,
                maxargs: $maxargs,
                name: $name
            };

            fn new(args: Vec<$crate::expressions::base::ExpressionType>, span: logos::Span) -> Result<Self, $crate::parse::ParserError> {
                if !Self::INFO.validate(args.len()) {
                    return Err($crate::ParserError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                Ok(Self {
                    span,
                    args,
                })
            }

            fn get_args(&'a self) -> Self::Iter<'a> {
                self.args.iter()
            }
        }
    }
}
