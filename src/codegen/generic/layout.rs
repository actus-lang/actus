use std::collections::{BTreeMap, HashMap};

use crate::ast::{
    BuiltinType, EnumDef, PackDecl, Program, StructDef, TopLevelDecl, TypeIdentity, TypeName,
    builtin_enum_definitions, lookup_builtin_type, primitive_type,
};
use crate::semantic::GenericInstance;

use super::super::native::NativeEmitError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericFieldLayout {
    pub(crate) name: Option<String>,
    pub(crate) offset: u32,
    pub(crate) size: u32,
    pub(crate) alignment: u32,
    pub(crate) owned: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericStructLayout {
    pub(crate) canonical_key: String,
    pub(crate) size: u32,
    pub(crate) alignment: u32,
    pub(crate) fields: Vec<GenericFieldLayout>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericEnumVariantLayout {
    pub(crate) name: String,
    pub(crate) discriminant: u32,
    pub(crate) fields: Vec<GenericFieldLayout>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GenericEnumLayout {
    pub(crate) canonical_key: String,
    pub(crate) size: u32,
    pub(crate) alignment: u32,
    pub(crate) payload_offset: u32,
    pub(crate) max_payload_size: u32,
    pub(crate) variants: Vec<GenericEnumVariantLayout>,
}

#[derive(Clone, Copy)]
pub(super) struct ValueLayout {
    pub(super) size: u32,
    pub(super) alignment: u32,
}

pub(crate) struct GenericLayoutRegistry {
    pub(super) structs: HashMap<String, StructDef>,
    pub(super) enums: HashMap<String, EnumDef>,
    pub(super) packs: HashMap<String, PackDecl>,
    pub(super) pointer_size: u32,
    struct_layouts: BTreeMap<TypeIdentity, GenericStructLayout>,
    enum_layouts: BTreeMap<TypeIdentity, GenericEnumLayout>,
}

impl GenericLayoutRegistry {
    pub(crate) fn from_program(
        program: &Program,
        instances: &[GenericInstance],
        pointer_size: u32,
    ) -> Result<Self, NativeEmitError> {
        let (structs, enums) = definitions(program);
        let mut registry = Self {
            structs,
            enums,
            packs: pack_definitions(program),
            pointer_size,
            struct_layouts: BTreeMap::new(),
            enum_layouts: BTreeMap::new(),
        };
        for instance in instances {
            registry.layout_instance(instance)?;
        }
        Ok(registry)
    }

    #[allow(dead_code)]
    pub(crate) fn struct_layout(&self, key: &TypeIdentity) -> Option<&GenericStructLayout> {
        self.struct_layouts.get(key)
    }

    #[allow(dead_code)]
    pub(crate) fn enum_layout(&self, key: &TypeIdentity) -> Option<&GenericEnumLayout> {
        self.enum_layouts.get(key)
    }

    fn layout_instance(&mut self, instance: &GenericInstance) -> Result<(), NativeEmitError> {
        if self.structs.contains_key(&instance.name) {
            let definition = self.structs.get(&instance.name).cloned().ok_or_else(|| {
                NativeEmitError(format!("missing generic struct `{}`", instance.name))
            })?;
            let layout = self.layout_struct(&definition, &instance.arguments, &mut Vec::new())?;
            self.struct_layouts.insert(instance.identity(), layout);
        } else if self.enums.contains_key(&instance.name) {
            let definition = self.enums.get(&instance.name).cloned().ok_or_else(|| {
                NativeEmitError(format!("missing generic enum `{}`", instance.name))
            })?;
            let layout = self.layout_enum(&definition, &instance.arguments, &mut Vec::new())?;
            self.enum_layouts.insert(instance.identity(), layout);
        }
        Ok(())
    }

    pub(super) fn layout_type(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Result<ValueLayout, NativeEmitError> {
        if type_name.reference_role.is_some() {
            return Ok(ValueLayout { size: self.pointer_size, alignment: self.pointer_size });
        }
        if type_name.name == "Array" {
            return self.layout_array(type_name, visiting);
        }
        if type_name.name == "Region" {
            return Ok(ValueLayout { size: 56, alignment: 8 });
        }
        if let Some(builtin) = lookup_builtin_type(&type_name.name) {
            return builtin_layout(builtin, self.pointer_size);
        }
        if let Some(primitive) = primitive_type(&type_name.name) {
            return primitive_layout(primitive);
        }
        if let Some(pack) = self.packs.get(&type_name.name) {
            return self.layout_type(pack.storage.type_name(), visiting);
        }
        if let Some(definition) = self.structs.get(&type_name.name) {
            let layout = self.layout_struct(definition, &type_name.arguments, visiting)?;
            return Ok(ValueLayout { size: layout.size, alignment: layout.alignment });
        }
        if let Some(definition) = self.enums.get(&type_name.name) {
            let layout = self.layout_enum(definition, &type_name.arguments, visiting)?;
            return Ok(ValueLayout { size: layout.size, alignment: layout.alignment });
        }
        Err(NativeEmitError(format!("unknown generic layout type `{}`", type_name.name)))
    }

    fn layout_array(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Result<ValueLayout, NativeEmitError> {
        let [element, capacity] = type_name.arguments.as_slice() else {
            return Err(NativeEmitError(
                "array layout requires an element type and capacity".to_owned(),
            ));
        };
        let capacity =
            capacity.name.parse::<u32>().ok().filter(|capacity| *capacity > 0).ok_or_else(
                || NativeEmitError("array layout requires a positive capacity".to_owned()),
            )?;
        let element_layout = self.layout_type(element, visiting)?;
        let size = element_layout
            .size
            .checked_mul(capacity)
            .ok_or_else(|| NativeEmitError("array layout size overflow".to_owned()))?;
        Ok(ValueLayout { size, alignment: element_layout.alignment })
    }
}

pub(super) fn definitions(
    program: &Program,
) -> (HashMap<String, StructDef>, HashMap<String, EnumDef>) {
    let mut structs = HashMap::new();
    let mut enums = builtin_enum_definitions()
        .into_iter()
        .map(|definition| (definition.name.clone(), definition))
        .collect::<HashMap<_, _>>();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Struct(definition) => {
                structs.insert(definition.name.clone(), definition.clone());
            }
            TopLevelDecl::Enum(definition) => {
                enums.insert(definition.name.clone(), definition.clone());
            }
            _ => {}
        }
    }
    (structs, enums)
}

fn pack_definitions(program: &Program) -> HashMap<String, PackDecl> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Pack(pack) => Some((pack.name.clone(), pack.clone())),
            _ => None,
        })
        .collect()
}

fn builtin_layout(builtin: BuiltinType, pointer_size: u32) -> Result<ValueLayout, NativeEmitError> {
    match builtin {
        BuiltinType::Int | BuiltinType::Bool => Ok(ValueLayout { size: 4, alignment: 4 }),
        BuiltinType::String | BuiltinType::Buffer => {
            Ok(ValueLayout { size: pointer_size, alignment: pointer_size })
        }
        BuiltinType::Array | BuiltinType::Map => Err(NativeEmitError(format!(
            "unsupported generic layout type `{}`",
            builtin.spec().name
        ))),
    }
}

fn primitive_layout(primitive: crate::ast::PrimitiveType) -> Result<ValueLayout, NativeEmitError> {
    let (size, alignment) = match primitive {
        crate::ast::PrimitiveType::Integer { width, .. } => match width {
            1..=8 => (1, 1),
            9..=16 => (2, 2),
            17..=32 => (4, 4),
            33..=64 => (8, 8),
            65..=128 => (16, 16),
            _ => return Err(NativeEmitError("invalid primitive integer width".to_owned())),
        },
        crate::ast::PrimitiveType::Float { width } => (u32::from(width / 8), u32::from(width / 8)),
        crate::ast::PrimitiveType::Void => (0, 1),
    };
    Ok(ValueLayout { size, alignment })
}

pub(super) fn application_key(name: &str, arguments: &[TypeName]) -> String {
    if arguments.is_empty() {
        return name.to_owned();
    }
    format!("{}[{}]", name, arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(","))
}

fn canonical_type_name(type_name: &TypeName) -> String {
    application_key(&type_name.name, &type_name.arguments)
}

pub(super) fn enter_layout(key: &str, visiting: &[String]) -> Result<(), NativeEmitError> {
    if visiting.iter().any(|candidate| candidate == key) {
        return Err(NativeEmitError(format!(
            "recursive generic layout for `{key}` is not supported"
        )));
    }
    Ok(())
}

pub(super) fn align_up(offset: u32, alignment: u32) -> u32 {
    offset.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::GenericLayoutRegistry;
    use crate::ast::{TypeIdentity, TypeName};
    use crate::lexer::scan;
    use crate::parser::parse;
    use crate::semantic::analyze;

    fn type_name(name: &str) -> TypeName {
        TypeName {
            name: name.to_owned(),
            arguments: Vec::new(),
            reference_role: None,
            span: crate::lexer::SourceSpan::new(0, 0),
        }
    }

    #[test]
    fn calculates_substituted_struct_offsets() {
        let (tokens, errors) =
            scan("struct Box[T] { item: T, } verb main(erg value: Box[Int]) { }");
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let semantic = analyze(&program).expect("source should analyze");
        let layouts = GenericLayoutRegistry::from_program(&program, &semantic.generic_instances, 8)
            .expect("generic layout should pass");
        let key = TypeIdentity::from_application("Box", &[type_name("Int")]);
        let layout = layouts.struct_layout(&key).expect("Box[Int] should be laid out");
        assert_eq!(layout.size, 4);
        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[0].size, 4);
    }

    #[test]
    fn calculates_nested_enum_payload_layout() {
        let (tokens, errors) = scan(
            "struct Box[T] { item: T, } enum Result[T] { Ok(T), } verb main(erg value: Result[Box[Int]]) { }",
        );
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let semantic = analyze(&program).expect("source should analyze");
        let layouts = GenericLayoutRegistry::from_program(&program, &semantic.generic_instances, 8)
            .expect("generic layout should pass");
        let box_type = TypeName {
            name: "Box".to_owned(),
            arguments: vec![type_name("Int")],
            reference_role: None,
            span: crate::lexer::SourceSpan::new(0, 0),
        };
        let key = TypeIdentity::from_application("Result", &[box_type]);
        let layout = layouts.enum_layout(&key).expect("Result[Box[Int]] should be laid out");
        assert_eq!(layout.payload_offset, 4);
        assert_eq!(layout.max_payload_size, 4);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.variants[0].fields[0].size, 4);
    }

    #[test]
    fn calculates_const_generic_array_field_layout() {
        let (tokens, errors) = scan(
            "struct Cell { erg charge: u8, } struct Fabric[N: Usize] { erg cells: Array[Cell, N], } verb main(erg fabric: Fabric[8]) { }",
        );
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let semantic = analyze(&program).expect("source should analyze");
        let layouts = GenericLayoutRegistry::from_program(&program, &semantic.generic_instances, 8)
            .expect("generic layout should pass");
        let key = TypeIdentity::from_application("Fabric", &[type_name("8")]);
        let layout = layouts.struct_layout(&key).expect("Fabric[8] should be laid out");
        assert_eq!(layout.size, 8);
        assert_eq!(layout.fields[0].size, 8);
    }

    #[test]
    fn calculates_const_generic_arrays_of_pack_elements() {
        let (tokens, errors) = scan(
            "pack Column { erg storage: u32; layout little; fields { erg marker: u32 at 0; } } struct Fabric[N: Usize] { erg columns: Array[Column, N], } verb main(erg fabric: Fabric[2]) { }",
        );
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let semantic = analyze(&program).expect("source should analyze");
        let layouts = GenericLayoutRegistry::from_program(&program, &semantic.generic_instances, 8)
            .expect("generic pack-array layout should pass");
        let key = TypeIdentity::from_application("Fabric", &[type_name("2")]);
        let layout = layouts.struct_layout(&key).expect("Fabric[2] should be laid out");
        assert_eq!(layout.size, 8);
        assert_eq!(layout.fields[0].size, 8);
    }
}
