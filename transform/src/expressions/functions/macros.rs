#[macro_export]
macro_rules! function_def {
    // Base, should have defined the struct
    (_display $typ:ident) => {
        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(", <Self as $crate::expressions::functions::FunctionExpression>::INFO.name)?;
                let mut is_first = true;
                for idx in 0..$crate::expressions::base::ExpressionMeta::num_children(self) {
                    let expr = $crate::expressions::base::ExpressionMeta::get_child(self, idx).unwrap();
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
    (_default_lambda $typ:ident) => {
        impl $crate::expressions::functions::LambdaAcceptFunction for $typ {}
    };
    ($typ:ident, $name:expr, $nargs:expr) => {
        function_def!(_default_lambda $typ);
        function_def!(_fixargs $typ, $name, $nargs);
    };
    ($typ:ident, $name:expr, $nargs:expr, lambda) => {
        function_def!(_fixargs $typ, $name, $nargs);
    };
    (_fixargs $typ:ident, $name:expr, $nargs:expr) => {
        #[derive(Debug, Clone)]
        pub struct $typ {
            args: [Box<$crate::expressions::base::ExpressionType>; $nargs],
            #[allow(dead_code)]
            span: logos::Span,
        }

        function_def!(_display $typ);

        impl $crate::expressions::functions::FunctionExpression for $typ {
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
                let num_args = args.len();
                for (idx, arg) in args.iter().enumerate() {
                    if let $crate::expressions::base::ExpressionType::Lambda(lambda) = arg {
                        <Self as $crate::expressions::functions::LambdaAcceptFunction>::validate_lambda(idx, lambda, num_args)?;
                    }
                }
                Ok(Self {
                    span,
                    args: args.into_iter().map(|a| Box::new(a)).collect::<Vec<_>>().try_into().unwrap()
                })
            }
        }

        impl $crate::expressions::base::ExpressionMeta for $typ {
            fn num_children(&self) -> usize {
                $nargs
            }

            fn get_child(&self, idx: usize) -> Option<&$crate::expressions::base::ExpressionType> {
                if idx > $nargs - 1 {
                    None
                } else {
                    Some(&self.args[idx])
                }
            }

            fn get_child_mut(&mut self, idx: usize) -> Option<&mut $crate::expressions::base::ExpressionType> {
                if idx > $nargs - 1 {
                    None
                } else {
                    Some(&mut self.args[idx])
                }
            }

            fn set_child(&mut self, idx: usize, item: $crate::expressions::base::ExpressionType) {
                if idx >= $nargs {
                    return;
                }
                self.args[idx] = Box::new(item);
            }
        }
    };
    ($typ:ident, $name:expr, $minargs:expr, $maxargs:expr) => {
        function_def!(_default_lambda $typ);
        function_def!(_dynargs $typ, $name, $minargs, $maxargs);
    };
    ($typ:ident, $name:expr, $minargs:expr, $maxargs:expr, lambda) => {
        function_def!(_dynargs $typ, $name, $minargs, $maxargs);
    };
    (_dynargs $typ:ident, $name:expr, $minargs:expr, $maxargs:expr) => {
        #[derive(Debug, Clone)]
        pub struct $typ {
            args: Vec<$crate::expressions::base::ExpressionType>,
            #[allow(dead_code)]
            span: logos::Span
        }

        function_def!(_display $typ);

        impl $crate::expressions::functions::FunctionExpression for $typ {
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
        }

        impl $crate::expressions::base::ExpressionMeta for $typ {
            fn num_children(&self) -> usize {
                self.args.len()
            }

            fn get_child(&self, idx: usize) -> Option<&$crate::expressions::base::ExpressionType> {
                self.args.get(idx)
            }

            fn get_child_mut(&mut self, idx: usize) -> Option<&mut $crate::expressions::base::ExpressionType> {
                self.args.get_mut(idx)
            }

            fn set_child(&mut self, idx: usize, item: $crate::expressions::base::ExpressionType) {
                if idx >= self.args.len() {
                    return;
                }
                self.args[idx] = item;
            }
        }
    }
}
