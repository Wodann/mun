use memory::gc;

/// Defines the garbage collector used by the `Runtime`.
pub type GarbageCollector<'a> = gc::MarkSweep<&'a abi::TypeInfo, gc::NoopObserver<gc::Event>>;

pub use gc::GcPtr;
pub type GcRootPtr<'a> = gc::GcRootPtr<&'a abi::TypeInfo, GarbageCollector<'a>>;
