use std::alloc::Layout;

mod cast;
pub mod diff;
pub mod gc;
pub mod mapping;

pub mod prelude {
    pub use crate::diff::{diff, Diff, FieldDiff, FieldEditKind};
    pub use crate::mapping::{Action, FieldMappingDesc};
}

/// A trait used to obtain a type's description.
pub trait TypeDesc: Send + Sync {
    /// Returns the name of this type.
    fn name(&self) -> &str;
    /// Returns the `Guid` of this type.
    fn guid(&self) -> &abi::Guid;
    /// Returns the `TypeGroup` of this type.
    fn group(&self) -> abi::TypeGroup;
}

/// A trait used to obtain a type's memory layout.
pub trait TypeLayout: Send + Sync {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;
}

/// A trait used to obtain a type's fields.
pub trait TypeFields<T>: Send + Sync {
    /// Returns the type's fields.
    fn fields(&self) -> Vec<(&str, T)>;
    /// Returns the type's fields' offsets.
    fn offsets(&self) -> &[u16];
}

impl<'t> TypeDesc for &'t abi::TypeInfo {
    fn name(&self) -> &str {
        abi::TypeInfo::name(self)
    }

    fn guid(&self) -> &abi::Guid {
        &self.guid
    }

    fn group(&self) -> abi::TypeGroup {
        self.group
    }
}

impl<'t> TypeFields<Self> for &'t abi::TypeInfo {
    fn fields(&self) -> Vec<(&str, Self)> {
        if let Some(s) = self.as_struct() {
            s.field_names()
                .zip(s.field_types().iter().cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    fn offsets(&self) -> &[u16] {
        if let Some(s) = self.as_struct() {
            s.field_offsets()
        } else {
            &[]
        }
    }
}

impl<'a> TypeLayout for &'a abi::TypeInfo {
    fn layout(&self) -> Layout {
        Layout::from_size_align(self.size_in_bytes(), self.alignment())
            .unwrap_or_else(|_| panic!("invalid layout from Mun Type: {:?}", self))
    }
}
