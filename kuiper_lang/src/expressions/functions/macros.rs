/// Define a function struct with the given name and argument count.
///
/// # Usage
///
/// - `function_def!(MyFunctionType, "my_function", 2)` defines a function type `MyFunctionType` that takes exactly 2 arguments and is called "my_function".
/// - `function_def!(MyFunctionType, "my_function", 1, None)` defines a function type that takes at least 1 argument and any number of arguments after that.
/// - `function_def!(MyFunctionType, "my_function", 1, Some(3))` defines a function type that takes between 1 and 3 arguments.
/// - `function_def!(MyFunctionType, "my_function", 2, lambda)` defines a function type that takes exactly 2 arguments
///   and also accepts a lambda as an argument. This requires you to implement the `LambdaAcceptFunction` trait for `MyFunctionType`, to validate which arguments
///   should accept lambdas, and to validate the number of arguments in the lambda itself.
///
#[macro_export]
macro_rules! function_def {
    // Base, should have defined the struct
    (_display $typ:ident) => {
        impl core::fmt::Display for $typ {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}(", <Self as $crate::functions::FunctionExpression>::INFO.name)?;
                let mut is_first = true;
                for item in &self.args {
                    if !is_first {
                        write!(f, ", ")?;
                    }
                    is_first = false;
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            }
        }
    };
    (_default_lambda $typ:ident) => {
        impl $crate::functions::LambdaAcceptFunction for $typ {}
    };
    ($typ:ident, $name:expr, $nargs:expr) => {
        function_def!(_default_lambda $typ);
        function_def!(_fixargs $typ, $name, $nargs);
    };
    ($typ:ident, $name:expr, $nargs:expr, lambda) => {
        function_def!(_fixargs $typ, $name, $nargs);
    };
    (_fixargs $typ:ident, $name:expr, $nargs:expr) => {
        #[derive(Debug)]
        pub struct $typ {
            args: [$crate::alloc::boxed::Box<$crate::ExpressionType>; $nargs],
            #[allow(dead_code)]
            span: logos::Span,
        }

        function_def!(_display $typ);

        impl $crate::functions::FunctionExpression for $typ {
            const INFO: $crate::functions::FunctionInfo = $crate::functions::FunctionInfo {
                minargs: $nargs,
                maxargs: Some($nargs),
                name: $name
            };

            fn new(args: $crate::alloc::vec::Vec<$crate::ExpressionType>, span: logos::Span) -> Result<Self, $crate::BuildError> {
                if !Self::INFO.validate(args.len()) {
                    return Err($crate::BuildError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                let num_args = args.len();
                for (idx, arg) in args.iter().enumerate() {
                    if let $crate::ExpressionType::Lambda(lambda) = arg {
                        <Self as $crate::functions::LambdaAcceptFunction>::validate_lambda(idx, lambda, num_args)?;
                    }
                }
                Ok(Self {
                    span,
                    args: args.into_iter().map(|a| $crate::alloc::boxed::Box::new(a)).collect::<$crate::alloc::vec::Vec<_>>().try_into().unwrap()
                })
            }
        }

        impl $crate::ExpressionMeta for $typ {
            fn iter_children_mut(&mut self) -> $crate::alloc::boxed::Box<dyn Iterator<Item = &mut $crate::ExpressionType> + '_> {
                $crate::alloc::boxed::Box::new(self.args.iter_mut().map(|m| m.as_mut()))
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
        #[derive(Debug)]
        pub struct $typ {
            args: $crate::alloc::vec::Vec<$crate::ExpressionType>,
            #[allow(dead_code)]
            span: logos::Span
        }

        function_def!(_display $typ);

        impl $crate::functions::FunctionExpression for $typ {
            const INFO: $crate::functions::FunctionInfo = $crate::functions::FunctionInfo {
                minargs: $minargs,
                maxargs: $maxargs,
                name: $name
            };

            fn new(args: $crate::alloc::vec::Vec<$crate::ExpressionType>, span: logos::Span) -> Result<Self, $crate::BuildError> {
                if !Self::INFO.validate(args.len()) {
                    return Err($crate::BuildError::n_function_args(
                        span,
                        &Self::INFO.num_args_desc(),
                    ));
                }
                let num_args = args.len();
                for (idx, arg) in args.iter().enumerate() {
                    if let $crate::ExpressionType::Lambda(lambda) = arg {
                        <Self as $crate::functions::LambdaAcceptFunction>::validate_lambda(idx, lambda, num_args)?;
                    }
                }
                Ok(Self {
                    span,
                    args,
                })
            }
        }

        impl $crate::ExpressionMeta for $typ {
            fn iter_children_mut(&mut self) -> $crate::alloc::boxed::Box<dyn Iterator<Item = &mut $crate::ExpressionType> + '_> {
                $crate::alloc::boxed::Box::new(self.args.iter_mut())
            }
        }
    }
}

pub use function_def;
