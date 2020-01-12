use crate::prelude::*;
use md5;

/// A type to emulate dynamic typing across compilation units for static types.
pub trait Reflection: 'static {
    /// Retrieves the type's `Guid`.
    fn type_guid() -> Guid {
        Guid {
            b: md5::compute(Self::type_name()).0,
        }
    }

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

/// A type to emulate dynamic typing across compilation units for statically typed values.
pub trait ArgumentReflection {
    /// The resulting type after dereferencing.
    type Marshalled: Sized;

    /// Retrieves the `Guid` of the value's type.
    fn type_guid(&self) -> Guid {
        Guid {
            b: md5::compute(self.type_name()).0,
        }
    }

    /// Retrieves the name of the value's type.
    fn type_name(&self) -> &str;

    /// Marshalls the value.
    fn marshall(self) -> Self::Marshalled;
}

impl<T: Reflection> ArgumentReflection for T {
    type Marshalled = Self;

    fn type_name(&self) -> &str {
        <T as Reflection>::type_name()
    }

    fn marshall(self) -> Self::Marshalled {
        self
    }
}

impl Reflection for f64 {
    fn type_name() -> &'static str {
        "@core::float"
    }
}

impl Reflection for i64 {
    fn type_name() -> &'static str {
        "@core::int"
    }
}

impl Reflection for bool {
    fn type_name() -> &'static str {
        "@core::bool"
    }
}

impl Reflection for () {
    fn type_name() -> &'static str {
        "@core::empty"
    }
}
