use crate::garbage_collector::{GcPtr, GcRootPtr};
use crate::{
    marshal::Marshal,
    reflection::{
        equals_argument_type, equals_return_type, ArgumentReflection, ReturnTypeReflection,
    },
    Runtime,
};
use memory::gc::{GcRuntime, HasIndirectionPtr};
use std::cell::RefCell;
use std::{
    ptr::{self, NonNull},
    rc::Rc,
};

/// Represents a Mun struct pointer.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawStruct(GcPtr);

impl RawStruct {
    /// Returns a pointer to the struct memory.
    pub unsafe fn get_ptr(&self) -> *const u8 {
        self.0.deref()
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct.
/// TODO: Handle destruction of `struct(value)`
pub struct StructRef<'r> {
    handle: GcRootPtr<'r>,
    runtime: Rc<RefCell<Runtime<'r>>>,
}

impl<'s> StructRef<'s> {
    /// Creates a `StructRef` that wraps a raw Mun struct.
    fn new(runtime: Rc<RefCell<Runtime<'s>>>, raw: RawStruct) -> Self {
        let handle = {
            let runtime_ref = runtime.borrow();
            assert!(runtime_ref.gc().ptr_type(raw.0).group.is_struct());

            GcRootPtr::new(&runtime_ref.gc, raw.0)
        };

        Self { runtime, handle }
    }

    /// Consumes the `StructRef`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        RawStruct(self.handle.handle())
    }

    /// Returns the type information of the struct.
    pub fn type_info<'t>(struct_ref: &Self, runtime_ref: &'t Runtime<'s>) -> &'t abi::TypeInfo {
        runtime_ref.gc.ptr_type(struct_ref.handle.handle())
    }

    ///
    ///
    /// # Safety
    ///
    ///
    unsafe fn field_offset_unchecked<T>(
        &self,
        struct_info: &abi::StructInfo,
        field_idx: usize,
    ) -> NonNull<T> {
        let offset = *struct_info.field_offsets().get_unchecked(field_idx);
        // self.raw is never null
        NonNull::new_unchecked(self.handle.deref::<u8>().add(offset as usize).cast::<T>() as *mut _)
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection<'s>>(&self, field_name: &str) -> Result<T, String> {
        let runtime_ref = self.runtime.borrow();
        let type_info = runtime_ref.gc.ptr_type(self.handle.handle());

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_return_type::<T>(field_type).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::Marshalled>(struct_info, field_idx) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime.clone(),
            Some(field_type),
        ))
    }

    /// Replaces the value of the field corresponding to the specified `field_name` and returns the
    /// old value.
    pub fn replace<T: ArgumentReflection<'s>>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<T, String> {
        let runtime_ref = self.runtime.borrow();
        let type_info = runtime_ref.gc.ptr_type(self.handle.handle());

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&runtime_ref, field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::Marshalled>(struct_info, field_idx) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime.clone(), Some(field_type));
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(field_type));
        Ok(old)
    }

    /// Sets the value of the field corresponding to the specified `field_name`.
    pub fn set<T: ArgumentReflection<'s>>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<(), String> {
        let runtime_ref = self.runtime.borrow();
        let type_info = runtime_ref.gc.ptr_type(self.handle.handle());

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&runtime_ref, field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::Marshalled>(struct_info, field_idx) };
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(field_type));
        Ok(())
    }
}

impl<'r> ArgumentReflection<'r> for StructRef<'r> {
    type Marshalled = RawStruct;

    fn type_guid(&self, runtime: &Runtime<'r>) -> abi::Guid {
        let type_info = runtime.gc().ptr_type(self.handle.handle());
        type_info.guid
    }

    fn type_name(&self, runtime: &Runtime<'r>) -> &str {
        let type_info = runtime.gc().ptr_type(self.handle.handle());
        type_info.name()
    }

    fn marshal(self) -> Self::Marshalled {
        self.into_raw()
    }
}

impl<'r> ReturnTypeReflection<'r> for StructRef<'r> {
    type Marshalled = RawStruct;

    fn type_name() -> &'static str {
        "struct"
    }
}

impl<'r> Marshal<'r, StructRef<'r>> for RawStruct {
    fn marshal_value(self, runtime: Rc<RefCell<Runtime<'r>>>) -> StructRef<'r> {
        StructRef::new(runtime, self)
    }

    fn marshal_from_ptr(
        ptr: NonNull<Self>,
        runtime: Rc<RefCell<Runtime<'r>>>,
        type_info: Option<&'r abi::TypeInfo>,
    ) -> StructRef<'r> {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();
        let struct_info = type_info.as_struct().unwrap();

        // Copy the contents of the struct based on what kind of pointer we are dealing with
        let gc_handle = if struct_info.memory_kind == abi::StructMemoryKind::Value {
            // For a value struct, `ptr` points to a struct value.

            // Create a new object using the runtime's intrinsic
            let mut gc_handle = {
                let runtime_ref = runtime.borrow();
                runtime_ref.gc().alloc(type_info)
            };

            // Construct
            let src = ptr.cast::<u8>().as_ptr() as *const _;
            let dest = unsafe { gc_handle.deref_mut::<u8>() };
            let size = type_info.size_in_bytes();
            unsafe { ptr::copy_nonoverlapping(src, dest, size as usize) };

            gc_handle
        } else {
            // For a gc struct, `ptr` points to a `GcPtr`.

            unsafe { *ptr.cast::<GcPtr>().as_ptr() }
        };

        StructRef::new(runtime, RawStruct(gc_handle))
    }

    fn marshal_to_ptr(value: RawStruct, mut ptr: NonNull<Self>, type_info: Option<&abi::TypeInfo>) {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();

        let struct_info = type_info.as_struct().unwrap();
        if struct_info.memory_kind == abi::StructMemoryKind::Value {
            let dest = ptr.cast::<u8>().as_ptr();
            let size = type_info.size_in_bytes();
            unsafe { ptr::copy_nonoverlapping(value.get_ptr(), dest, size as usize) };
        } else {
            unsafe { *ptr.as_mut() = value };
        }
    }
}
