use crate::{marshal::Marshal, Runtime, StructRef};
use abi::HasStaticTypeInfo;

/// Returns whether the specified argument type matches the `type_info`.
pub fn equals_argument_type<'r, 'e, 'f, T: ArgumentReflection<'r>>(
    runtime: &'f Runtime<'r>,
    type_info: &'e abi::TypeInfo,
    arg: &'f T,
) -> Result<(), (&'e str, &'f str)> {
    if type_info.guid != arg.type_guid(runtime) {
        Err((type_info.name(), arg.type_name(runtime)))
    } else {
        Ok(())
    }
}

/// Returns whether the specified return type matches the `type_info`.
pub fn equals_return_type<'r, T: ReturnTypeReflection<'r>>(
    type_info: &abi::TypeInfo,
) -> Result<(), (&str, &str)> {
    match type_info.group {
        abi::TypeGroup::FundamentalTypes => {
            if type_info.guid != T::type_guid() {
                return Err((type_info.name(), T::type_name()));
            }
        }
        abi::TypeGroup::StructTypes => {
            if <StructRef as ReturnTypeReflection>::type_guid() != T::type_guid() {
                return Err(("struct", T::type_name()));
            }
        }
    }
    Ok(())
}

/// A type to emulate dynamic typing across compilation units for static types.
pub trait ReturnTypeReflection<'r>: Sized + 'r {
    /// The resulting type after marshaling.
    type Marshalled: Marshal<'r, Self>;

    /// Retrieves the type's `Guid`.
    fn type_guid() -> abi::Guid {
        abi::Guid {
            b: md5::compute(Self::type_name()).0,
        }
    }

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection<'r>: Sized + 'r {
    /// The resulting type after dereferencing.
    type Marshalled: Marshal<'r, Self>;

    /// Retrieves the `Guid` of the value's type.
    fn type_guid(&self, runtime: &Runtime<'r>) -> abi::Guid;

    /// Retrieves the name of the value's type.
    fn type_name<'s>(&'s self, runtime: &'s Runtime<'r>) -> &'s str;

    /// Marshals the value.
    fn marshal(self) -> Self::Marshalled;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl<'r> ArgumentReflection<'r> for $ty {
                type Marshalled = Self;

                fn type_guid(&self, _runtime: &Runtime<'r>) -> abi::Guid {
                    Self::type_info().guid
                }

                fn type_name(&self, _runtime: &Runtime<'r>) -> &str {
                    Self::type_info().name()
                }

                fn marshal(self) -> Self::Marshalled {
                    self
                }
            }

            impl<'r> ReturnTypeReflection<'r> for $ty {
                type Marshalled = Self;

                fn type_guid() -> abi::Guid {
                    Self::type_info().guid
                }

                fn type_name() -> &'static str {
                    Self::type_info().name()
                }
            }
        )+
    }
}

impl_primitive_type!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool
);

impl<'r> ReturnTypeReflection<'r> for () {
    type Marshalled = ();

    fn type_name() -> &'static str {
        "core::empty"
    }
}

impl<'r, T> ArgumentReflection<'r> for *const T
where
    *const T: HasStaticTypeInfo,
    T: 'r,
{
    type Marshalled = Self;

    fn type_guid(&self, _runtime: &Runtime) -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name(&self, _runtime: &Runtime) -> &str {
        Self::type_info().name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl<'r, T> ReturnTypeReflection<'r> for *const T
where
    *const T: HasStaticTypeInfo,
    T: 'r,
{
    type Marshalled = Self;

    fn type_guid() -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}

impl<'r, T> ArgumentReflection<'r> for *mut T
where
    *mut T: HasStaticTypeInfo,
    T: 'r,
{
    type Marshalled = Self;

    fn type_guid(&self, _runtime: &Runtime) -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name(&self, _runtime: &Runtime) -> &str {
        Self::type_info().name()
    }

    fn marshal(self) -> Self::Marshalled {
        self
    }
}

impl<'r, T> ReturnTypeReflection<'r> for *mut T
where
    *mut T: HasStaticTypeInfo,
    T: 'r,
{
    type Marshalled = Self;

    fn type_guid() -> abi::Guid {
        Self::type_info().guid
    }

    fn type_name() -> &'static str {
        Self::type_info().name()
    }
}
