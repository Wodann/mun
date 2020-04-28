#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! count_args {
    () => { 0 };
    ($name:ident) => { 1 };
    ($first:ident, $($rest:ident),*) => {
        1 + count_args!($($rest),*)
    }
}

macro_rules! invoke_fn_impl {
    ($(
        fn $FnName:ident($($Arg:tt: $T:ident),*) -> $ErrName:ident;
    )+) => {
        $(
            /// An invocation error that contains the function name, a mutable reference to the
            /// runtime, passed arguments, and the output type. This allows the caller to retry
            /// the function invocation using the `Retriable` trait.
            pub struct $ErrName<'r, 's, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>> {
                msg: String,
                runtime: std::rc::Rc<core::cell::RefCell<Runtime<'r>>>,
                function_name: &'s str,
                $($Arg: $T,)*
                output: core::marker::PhantomData<Output>,
            }

            impl<'r, 's, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>> core::fmt::Debug for $ErrName<'r, 's, $($T,)* Output> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'r, 's, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>> core::fmt::Display for $ErrName<'r, 's, $($T,)* Output> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", &self.msg)
                }
            }

            impl<'r, 's, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>> std::error::Error for $ErrName<'r, 's, $($T,)* Output> {
                fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                    None
                }
            }

            impl<'r, 's, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>> $ErrName<'r, 's, $($T,)* Output> {
                /// Constructs a new invocation error.
                #[allow(clippy::too_many_arguments)]
                pub fn new(err_msg: String, runtime: std::rc::Rc<core::cell::RefCell<Runtime<'r>>>, function_name: &'s str, $($Arg: $T),*) -> Self {
                    Self {
                        msg: err_msg,
                        runtime,
                        function_name,
                        $($Arg,)*
                        output: core::marker::PhantomData,
                    }
                }
            }

            impl<'r, 's, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>> $crate::RetryResultExt for core::result::Result<Output, $ErrName<'r, 's, $($T,)* Output>> {
                type Output = Output;

                fn retry(self) -> Self {
                    match self {
                        Ok(output) => Ok(output),
                        Err(err) => {
                            eprintln!("{}", err.msg);
                            while !err.runtime.borrow_mut().update() {
                                // Wait until there has been an update that might fix the error
                            }
                            $crate::Runtime::$FnName(&err.runtime, err.function_name, $(err.$Arg,)*)
                        }
                    }
                }

                fn wait(mut self) -> Self::Output {
                    loop {
                        if let Ok(output) = self {
                            return output;
                        } else {
                            self = self.retry();
                        }
                    }
                }
            }

            impl<'r> Runtime<'r> {
                /// Invokes the method `method_name` with arguments `args`, in the library compiled
                /// based on the manifest at `manifest_path`.
                ///
                /// If an error occurs when invoking the method, an error message is logged. The
                /// runtime continues looping until the cause of the error has been resolved.
                #[allow(clippy::too_many_arguments, unused_assignments)]
                pub fn $FnName<'s, $($T: ArgumentReflection<'r>,)* Output: ReturnTypeReflection<'r>>(
                    runtime: &std::rc::Rc<core::cell::RefCell<Self>>,
                    function_name: &'s str,
                    $($Arg: $T,)*
                ) -> core::result::Result<Output, $ErrName<'r, 's, $($T,)* Output>> {
                    let runtime_ref = runtime.borrow();
                    match runtime_ref
                        .get_function_info(function_name)
                        .ok_or_else(|| format!("Failed to obtain function '{}'", function_name))
                        .and_then(|function_info| {
                            // Validate function signature
                            let num_args = $crate::count_args!($($T),*);

                            let arg_types = function_info.signature.arg_types();
                            if arg_types.len() != num_args {
                                return Err(format!(
                                    "Invalid number of arguments. Expected: {}. Found: {}.",
                                    arg_types.len(),
                                    num_args,
                                ));
                            }

                            #[allow(unused_mut, unused_variables)]
                            let mut idx = 0;
                            $(
                                crate::reflection::equals_argument_type(&runtime_ref, &arg_types[idx], &$Arg)
                                    .map_err(|(expected, found)| {
                                        format!(
                                            "Invalid argument type at index {}. Expected: {}. Found: {}.",
                                            idx,
                                            expected,
                                            found,
                                        )
                                    })?;
                                idx += 1;
                            )*

                            if let Some(return_type) = function_info.signature.return_type() {
                                crate::reflection::equals_return_type::<Output>(return_type)
                            } else if <() as ReturnTypeReflection>::type_guid() != Output::type_guid() {
                                Err((<() as ReturnTypeReflection>::type_name(), Output::type_name()))
                            } else {
                                Ok(())
                            }.map_err(|(expected, found)| {
                                format!(
                                    "Invalid return type. Expected: {}. Found: {}",
                                    expected,
                                    found,
                                )
                            })?;

                            Ok(function_info)
                        }) {
                        Ok(function_info) => {
                            let function: fn($($T::Marshalled),*) -> Output::Marshalled = unsafe {
                                core::mem::transmute(function_info.fn_ptr)
                            };
                            let result = function($($Arg.marshal()),*);

                            // Marshall the result
                            return Ok(result.marshal_value(runtime.clone()))
                        }
                        Err(e) => Err($ErrName::new(e, runtime.clone(), function_name, $($Arg),*))
                    }
                }
            }
        )+
    }
}

/// Invokes a runtime function and returns a [`Result`] that implements the [`RetryResultExt`]
/// trait.
///
/// The first argument `invoke_fn` receives is a `Runtime` and the second argument is a function
/// string. This must be a `&str`.
///
/// Additional parameters passed to `invoke_fn` are the arguments of the function in the order
/// given.
#[macro_export]
macro_rules! invoke_fn {
    ($Runtime:expr, $FnName:expr) => {
        $crate::Runtime::invoke_fn0(&$Runtime, $FnName)
    };
    ($Runtime:expr, $FnName:expr, $A:expr) => {
        $crate::Runtime::invoke_fn1(&$Runtime, $FnName, $A)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr) => {
        $crate::Runtime::invoke_fn2(&$Runtime, $FnName, $A, $B)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr) => {
        $crate::Runtime::invoke_fn3(&$Runtime, $FnName, $A, $B, $C)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr) => {
        $crate::Runtime::invoke_fn4(&$Runtime, $FnName, $A, $B, $C, $D)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr) => {
        $crate::Runtime::invoke_fn5(&$Runtime, $FnName, $A, $B, $C, $D, $E)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr) => {
        $crate::Runtime::invoke_fn6(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr) => {
        $crate::Runtime::invoke_fn7(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr) => {
        $crate::Runtime::invoke_fn8(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr) => {
        $crate::Runtime::invoke_fn9(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr) => {
        $crate::Runtime::invoke_fn10(&$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J)
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr) => {
        $crate::Runtime::invoke_fn11(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr) => {
        $crate::Runtime::invoke_fn12(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr, $M:expr) => {
        $crate::Runtime::invoke_fn13(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr, $M:expr, $N:expr) => {
        $crate::Runtime::invoke_fn14(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M, $N,
        )
    };
    ($Runtime:expr, $FnName:expr, $A:expr, $B:expr, $C:expr, $D:expr, $E:expr, $F:expr, $G:expr, $H:expr, $I:expr, $J:expr, $K:expr, $L:expr, $M:expr, $N:expr, $O:expr) => {
        $crate::Runtime::invoke_fn15(
            &$Runtime, $FnName, $A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M, $N, $O,
        )
    };
}
