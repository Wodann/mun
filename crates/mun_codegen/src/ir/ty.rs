use super::try_convert_any_to_basic;
use crate::IrDatabase;
use abi::Guid;
use hir::{ApplicationTy, CallableDef, Ty, TypeCtor};
use inkwell::types::{AnyTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::AddressSpace;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeGroup {
    FundamentalTypes,
    StructTypes(hir::Struct, StructType),
}

impl From<TypeGroup> for u64 {
    fn from(group: TypeGroup) -> Self {
        match group {
            TypeGroup::FundamentalTypes => 0,
            TypeGroup::StructTypes(_, _) => 1,
        }
    }
}

#[derive(Clone, Debug, Eq)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: String,
    pub group: TypeGroup,
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.guid.b)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl TypeInfo {
    fn new<S: AsRef<str>>(name: S, group: TypeGroup) -> TypeInfo {
        TypeInfo {
            name: name.as_ref().to_string(),
            guid: Guid {
                b: md5::compute(name.as_ref()).0,
            },
            group,
        }
    }
}

/// Given a mun type, construct an LLVM IR type
pub(crate) fn ir_query(db: &impl IrDatabase, ty: Ty) -> AnyTypeEnum {
    let context = db.context();
    match ty {
        Ty::Empty => AnyTypeEnum::StructType(context.struct_type(&[], false)),
        Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
            TypeCtor::Float => AnyTypeEnum::FloatType(context.f64_type()),
            TypeCtor::Int => AnyTypeEnum::IntType(context.i64_type()),
            TypeCtor::Bool => AnyTypeEnum::IntType(context.bool_type()),
            TypeCtor::FnDef(def @ CallableDef::Function(_)) => {
                let ty = db.callable_sig(def);
                let params: Vec<BasicTypeEnum> = ty
                    .params()
                    .iter()
                    .map(|p| try_convert_any_to_basic(db.type_ir(p.clone())).unwrap())
                    .collect();

                let fn_type = match ty.ret() {
                    Ty::Empty => context.void_type().fn_type(&params, false),
                    ty => try_convert_any_to_basic(db.type_ir(ty.clone()))
                        .expect("could not convert return value")
                        .fn_type(&params, false),
                };

                AnyTypeEnum::FunctionType(fn_type)
            }
            TypeCtor::Struct(s) => {
                let struct_ty = db.struct_ty(s);
                match s.data(db).memory_kind {
                    hir::StructMemoryKind::GC => struct_ty.ptr_type(AddressSpace::Generic).into(),
                    hir::StructMemoryKind::Value => struct_ty.into(),
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!("unknown type can not be converted"),
    }
}

/// Returns the LLVM IR type of the specified struct
pub fn struct_ty_query(db: &impl IrDatabase, s: hir::Struct) -> StructType {
    let name = s.name(db).to_string();
    let struct_type = db.context().opaque_struct_type(&name);
    if struct_type.is_opaque() {
        let field_types: Vec<BasicTypeEnum> = s
            .fields(db)
            .iter()
            .map(|field| {
                let field_type = field.ty(db);
                try_convert_any_to_basic(db.type_ir(field_type))
                    .expect("could not convert field type")
            })
            .collect();

        struct_type.set_body(&field_types, false);
    }
    struct_type
}

/// Constructs the `TypeInfo` for the specified HIR type
pub fn type_info_query(db: &impl IrDatabase, ty: Ty) -> TypeInfo {
    match ty {
        Ty::Apply(ctor) => match ctor.ctor {
            TypeCtor::Float => TypeInfo::new("@core::float", TypeGroup::FundamentalTypes),
            TypeCtor::Int => TypeInfo::new("@core::int", TypeGroup::FundamentalTypes),
            TypeCtor::Bool => TypeInfo::new("@core::bool", TypeGroup::FundamentalTypes),
            TypeCtor::Struct(s) => TypeInfo::new(
                s.name(db).to_string(),
                TypeGroup::StructTypes(s, db.struct_ty(s)),
            ),
            _ => unreachable!("{:?} unhandled", ctor),
        },
        _ => unreachable!(),
    }
}
