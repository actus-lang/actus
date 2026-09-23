use std::collections::{BTreeMap, HashMap};

use crate::ast::{
    BuiltinType, EnumDef, EnumPayload, Program, StructDef, StructFieldRole, TopLevelDecl, TypeName,
    lookup_builtin_type,
};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::native::NativeEmitError;

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
struct ValueLayout {
    size: u32,
    alignment: u32,
}

pub(crate) struct GenericLayoutRegistry {
    structs: HashMap<String, StructDef>,
    enums: HashMap<String, EnumDef>,
    pointer_size: u32,
    struct_layouts: BTreeMap<String, GenericStructLayout>,
    enum_layouts: BTreeMap<String, GenericEnumLayout>,
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
            pointer_size,
            struct_layouts: BTreeMap::new(),
            enum_layouts: BTreeMap::new(),
        };
        for instance in instances {
            registry.layout_instance(instance)?;
        }
        Ok(registry)
    }

    // Consumed by the forthcoming generic lowering pass.
    #[allow(dead_code)]
    pub(crate) fn struct_layout(&self, key: &str) -> Option<&GenericStructLayout> {
        self.struct_layouts.get(key)
    }

    // Consumed by the forthcoming generic lowering pass.
    #[allow(dead_code)]
    pub(crate) fn enum_layout(&self, key: &str) -> Option<&GenericEnumLayout> {
        self.enum_layouts.get(key)
    }

    fn layout_instance(&mut self, instance: &GenericInstance) -> Result<(), NativeEmitError> {
        if self.structs.contains_key(&instance.name) {
            let definition = self.structs.get(&instance.name).cloned().ok_or_else(|| {
                NativeEmitError(format!("missing generic struct `{}`", instance.name))
            })?;
            let layout = self.layout_struct(&definition, &instance.arguments, &mut Vec::new())?;
            self.struct_layouts.insert(layout.canonical_key.clone(), layout);
        } else if self.enums.contains_key(&instance.name) {
            let definition = self.enums.get(&instance.name).cloned().ok_or_else(|| {
                NativeEmitError(format!("missing generic enum `{}`", instance.name))
            })?;
            let layout = self.layout_enum(&definition, &instance.arguments, &mut Vec::new())?;
            self.enum_layouts.insert(layout.canonical_key.clone(), layout);
        }
        Ok(())
    }

    fn layout_struct(
        &self,
        definition: &StructDef,
        arguments: &[TypeName],
        visiting: &mut Vec<String>,
    ) -> Result<GenericStructLayout, NativeEmitError> {
        let canonical_key = application_key(&definition.name, arguments);
        enter_layout(&canonical_key, visiting)?;
        let substitution = TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            arguments,
            definition.span,
        )
        .map_err(|error| NativeEmitError(format!("generic substitution failed: {error:?}")))?;
        let result = self.layout_struct_fields(definition, &substitution, visiting);
        visiting.pop();
        result.map(|(size, alignment, fields)| GenericStructLayout {
            canonical_key,
            size,
            alignment,
            fields,
        })
    }

    fn layout_struct_fields(
        &self,
        definition: &StructDef,
        substitution: &TypeSubstitution,
        visiting: &mut Vec<String>,
    ) -> Result<(u32, u32, Vec<GenericFieldLayout>), NativeEmitError> {
        let mut fields = Vec::new();
        let mut offset = 0;
        let mut alignment = 1;
        for field in &definition.fields {
            let field_type = substitution.apply(&field.ty);
            let layout = self.layout_type(&field_type, visiting)?;
            offset = align_up(offset, layout.alignment);
            fields.push(GenericFieldLayout {
                name: Some(field.name.clone()),
                offset,
                size: layout.size,
                alignment: layout.alignment,
                owned: matches!(field.role, StructFieldRole::Erg),
            });
            offset += layout.size;
            alignment = alignment.max(layout.alignment);
        }
        Ok((align_up(offset, alignment), alignment, fields))
    }

    fn layout_enum(
        &self,
        definition: &EnumDef,
        arguments: &[TypeName],
        visiting: &mut Vec<String>,
    ) -> Result<GenericEnumLayout, NativeEmitError> {
        let canonical_key = application_key(&definition.name, arguments);
        enter_layout(&canonical_key, visiting)?;
        let substitution = TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            arguments,
            definition.span,
        )
        .map_err(|error| NativeEmitError(format!("generic substitution failed: {error:?}")))?;
        let result = self.layout_enum_variants(definition, &substitution, visiting);
        visiting.pop();
        result.map(|(size, alignment, payload_offset, max_payload_size, variants)| {
            GenericEnumLayout {
                canonical_key,
                size,
                alignment,
                payload_offset,
                max_payload_size,
                variants,
            }
        })
    }

    fn layout_enum_variants(
        &self,
        definition: &EnumDef,
        substitution: &TypeSubstitution,
        visiting: &mut Vec<String>,
    ) -> Result<(u32, u32, u32, u32, Vec<GenericEnumVariantLayout>), NativeEmitError> {
        let mut variants = Vec::new();
        let mut max_payload_size = 0;
        let mut max_payload_alignment = 1;
        for (index, variant) in definition.variants.iter().enumerate() {
            let mut fields = Vec::new();
            let mut offset = 0;
            let mut payload_alignment = 1;
            for (name, field_type, owned) in variant_fields(&variant.payload) {
                let field_type = substitution.apply(field_type);
                let layout = self.layout_type(&field_type, visiting)?;
                offset = align_up(offset, layout.alignment);
                fields.push(GenericFieldLayout {
                    name,
                    offset,
                    size: layout.size,
                    alignment: layout.alignment,
                    owned,
                });
                offset += layout.size;
                payload_alignment = payload_alignment.max(layout.alignment);
            }
            let payload_size = align_up(offset, payload_alignment);
            max_payload_size = max_payload_size.max(payload_size);
            max_payload_alignment = max_payload_alignment.max(payload_alignment);
            variants.push(GenericEnumVariantLayout {
                name: variant.name.clone(),
                discriminant: u32::try_from(index)
                    .map_err(|_| NativeEmitError("enum has too many variants".to_owned()))?,
                fields,
            });
        }
        let payload_offset = align_up(4, max_payload_alignment);
        let alignment = 4.max(max_payload_alignment);
        let size = align_up(payload_offset + max_payload_size, alignment);
        Ok((size, alignment, payload_offset, max_payload_size, variants))
    }

    fn layout_type(
        &self,
        type_name: &TypeName,
        visiting: &mut Vec<String>,
    ) -> Result<ValueLayout, NativeEmitError> {
        if let Some(builtin) = lookup_builtin_type(&type_name.name) {
            return builtin_layout(builtin, self.pointer_size);
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
}

fn definitions(program: &Program) -> (HashMap<String, StructDef>, HashMap<String, EnumDef>) {
    let mut structs = HashMap::new();
    let mut enums = HashMap::new();
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

fn variant_fields(payload: &EnumPayload) -> Vec<(Option<String>, &TypeName, bool)> {
    match payload {
        EnumPayload::Unit => Vec::new(),
        EnumPayload::Tuple(types) => types.iter().map(|ty| (None, ty, false)).collect(),
        EnumPayload::Struct(fields) => {
            fields.iter().map(|field| (Some(field.name.clone()), &field.ty, false)).collect()
        }
    }
}

fn application_key(name: &str, arguments: &[TypeName]) -> String {
    if arguments.is_empty() {
        return name.to_owned();
    }
    format!("{}[{}]", name, arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(","))
}

fn canonical_type_name(type_name: &TypeName) -> String {
    application_key(&type_name.name, &type_name.arguments)
}

fn enter_layout(key: &str, visiting: &[String]) -> Result<(), NativeEmitError> {
    if visiting.iter().any(|candidate| candidate == key) {
        return Err(NativeEmitError(format!(
            "recursive generic layout for `{key}` is not supported"
        )));
    }
    Ok(())
}

fn align_up(offset: u32, alignment: u32) -> u32 {
    offset.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::GenericLayoutRegistry;
    use crate::lexer::scan;
    use crate::parser::parse;
    use crate::semantic::analyze;

    #[test]
    fn calculates_substituted_struct_offsets() {
        let (tokens, errors) =
            scan("struct Box[T] { item: T, } verb main(erg value: Box[Int]) { }");
        assert!(errors.is_empty());
        let program = parse(tokens).expect("source should parse");
        let semantic = analyze(&program).expect("source should analyze");
        let layouts = GenericLayoutRegistry::from_program(&program, &semantic.generic_instances, 8)
            .expect("generic layout should pass");
        let layout = layouts.struct_layout("Box[Int]").expect("Box[Int] should be laid out");
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
        let layout =
            layouts.enum_layout("Result[Box[Int]]").expect("Result[Box[Int]] should be laid out");
        assert_eq!(layout.payload_offset, 4);
        assert_eq!(layout.max_payload_size, 4);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.variants[0].fields[0].size, 4);
    }
}
