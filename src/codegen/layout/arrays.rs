use crate::ast::{EnumDef, EnumPayload, Expr, Program, Stmt, StructDef, TopLevelDecl, TypeName};

use super::super::generic::canonical_type_name;
use super::{ArrayLayout, LayoutRegistry};
use crate::codegen::native::NativeEmitError;

pub(crate) fn array_definitions(
    program: &Program,
    specialized_structs: &[StructDef],
    specialized_enums: &[EnumDef],
) -> Vec<TypeName> {
    let mut definitions = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => collect_verb_arrays(verb, &mut definitions),
            TopLevelDecl::ExternalVerb(verb) => collect_external_arrays(verb, &mut definitions),
            TopLevelDecl::Struct(structure) => structure
                .fields
                .iter()
                .for_each(|field| collect_type_arrays(&field.ty, &mut definitions)),
            TopLevelDecl::Enum(enumeration) => {
                for variant in &enumeration.variants {
                    match &variant.payload {
                        EnumPayload::Tuple(types) => {
                            types.iter().for_each(|ty| collect_type_arrays(ty, &mut definitions))
                        }
                        EnumPayload::Struct(fields) => fields
                            .iter()
                            .for_each(|field| collect_type_arrays(&field.ty, &mut definitions)),
                        EnumPayload::Unit => {}
                    }
                }
            }
            TopLevelDecl::Pack(pack) => {
                collect_type_arrays(pack.storage.type_name(), &mut definitions)
            }
            _ => {}
        }
    }
    for structure in specialized_structs {
        structure.fields.iter().for_each(|field| collect_type_arrays(&field.ty, &mut definitions));
    }
    for enumeration in specialized_enums {
        for variant in &enumeration.variants {
            match &variant.payload {
                EnumPayload::Tuple(types) => {
                    types.iter().for_each(|ty| collect_type_arrays(ty, &mut definitions))
                }
                EnumPayload::Struct(fields) => {
                    fields.iter().for_each(|field| collect_type_arrays(&field.ty, &mut definitions))
                }
                EnumPayload::Unit => {}
            }
        }
    }
    definitions
}

fn collect_verb_arrays(verb: &crate::ast::VerbDecl, definitions: &mut Vec<TypeName>) {
    verb.params.iter().for_each(|parameter| collect_type_arrays(&parameter.ty, definitions));
    if let Some(return_type) = &verb.return_type {
        collect_type_arrays(&return_type.ty, definitions);
    }
    collect_block_arrays(&verb.body, definitions);
}

fn collect_external_arrays(verb: &crate::ast::ExternalVerbDecl, definitions: &mut Vec<TypeName>) {
    verb.params.iter().for_each(|parameter| collect_type_arrays(&parameter.ty, definitions));
    if let Some(return_type) = &verb.return_type {
        collect_type_arrays(&return_type.ty, definitions);
    }
}

fn collect_type_arrays(type_name: &TypeName, definitions: &mut Vec<TypeName>) {
    if type_name.name == "Array"
        && type_name.arguments.len() == 2
        && type_name.arguments[1].name.parse::<u32>().is_ok()
    {
        let canonical = canonical_type_name(type_name);
        if !definitions.iter().any(|candidate| canonical_type_name(candidate) == canonical) {
            definitions.push(type_name.clone());
        }
    }
    type_name.arguments.iter().for_each(|argument| collect_type_arrays(argument, definitions));
}

fn collect_block_arrays(block: &crate::ast::Block, definitions: &mut Vec<TypeName>) {
    for statement in &block.statements {
        match statement {
            Stmt::OwnerDecl { ty, initializer, .. } => {
                if let Some(ty) = ty.as_deref().and_then(parse_array_type_name) {
                    collect_type_arrays(&ty, definitions);
                }
                collect_expression_arrays(initializer, definitions);
            }
            Stmt::Assignment { target, value, .. } => {
                collect_expression_arrays(&target.to_expr(), definitions);
                collect_expression_arrays(value, definitions);
            }
            Stmt::Expression { expression: value, .. }
            | Stmt::Return { value: Some(value), .. } => {
                collect_expression_arrays(value, definitions);
            }
            Stmt::CompoundAssignment { target, value, .. } => {
                collect_expression_arrays(&target.to_expr(), definitions);
                collect_expression_arrays(value, definitions);
            }
            Stmt::Loop(nested) | Stmt::Block(nested) => collect_block_arrays(nested, definitions),
            Stmt::If { condition, then_branch, else_branch, .. } => {
                collect_expression_arrays(condition, definitions);
                collect_block_arrays(then_branch, definitions);
                if let Some(crate::ast::IfBranch::Block(block)) = else_branch {
                    collect_block_arrays(block, definitions);
                }
            }
            Stmt::Return { value: None, .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Drop { .. } => {}
        }
    }
}

fn collect_expression_arrays(expression: &Expr, definitions: &mut Vec<TypeName>) {
    match expression {
        Expr::Call { callee, arguments, .. } => {
            if let Some(array) = parse_array_type_name(callee) {
                collect_type_arrays(&array, definitions);
            }
            arguments
                .iter()
                .for_each(|argument| collect_expression_arrays(&argument.expression, definitions));
        }
        Expr::Index { target, index, .. } => {
            collect_expression_arrays(target, definitions);
            collect_expression_arrays(index, definitions);
        }
        Expr::FieldAccess { object, .. }
        | Expr::Cast { expression: object, .. }
        | Expr::Borrow { expression: object, .. }
        | Expr::Grouping { expression: object, .. }
        | Expr::Unary { expression: object, .. }
        | Expr::Try { expression: object, .. } => collect_expression_arrays(object, definitions),
        Expr::Binary { left, right, .. } => {
            collect_expression_arrays(left, definitions);
            collect_expression_arrays(right, definitions);
        }
        Expr::MethodCall { receiver, arguments, .. } => {
            collect_expression_arrays(receiver, definitions);
            arguments
                .iter()
                .for_each(|argument| collect_expression_arrays(&argument.expression, definitions));
        }
        Expr::StructLit { fields, .. } => {
            fields.iter().for_each(|field| collect_expression_arrays(&field.value, definitions))
        }
        Expr::Case { subject, branches, .. } => {
            collect_expression_arrays(subject, definitions);
            branches.iter().for_each(|branch| match &branch.body {
                crate::ast::CaseBody::Expression(value) => {
                    collect_expression_arrays(value, definitions)
                }
                crate::ast::CaseBody::Block(block) => collect_block_arrays(block, definitions),
            });
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            collect_expression_arrays(condition, definitions);
            collect_block_arrays(then_branch, definitions);
            if let Some(crate::ast::IfBranch::Block(block)) = else_branch {
                collect_block_arrays(block, definitions);
            }
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. }
        | Expr::BufferLiteral { .. } => {}
    }
}

fn parse_array_type_name(text: &str) -> Option<TypeName> {
    let inner = text.strip_prefix("Array[")?.strip_suffix(']')?;
    let (element, capacity) = inner.split_once(',')?;
    let span = crate::lexer::SourceSpan::new(0, 0);
    Some(TypeName {
        name: "Array".to_owned(),
        arguments: vec![
            TypeName {
                name: element.trim().to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span,
            },
            TypeName {
                name: capacity.trim().to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span,
            },
        ],
        reference_role: None,
        span,
    })
}

impl LayoutRegistry {
    pub(super) fn array_layout_for(
        &self,
        type_name: &TypeName,
    ) -> Result<ArrayLayout, NativeEmitError> {
        let element_name = type_name
            .arguments
            .first()
            .ok_or_else(|| NativeEmitError("array type has no element type".to_owned()))?;
        let capacity = type_name
            .arguments
            .get(1)
            .and_then(|capacity| capacity.name.parse::<u32>().ok())
            .ok_or_else(|| NativeEmitError("array type has no numeric capacity".to_owned()))?;
        let element = self.type_for_type_name(element_name).ok_or_else(|| {
            NativeEmitError(format!(
                "unknown array element `{}`",
                canonical_type_name(element_name)
            ))
        })?;
        let element_size = self
            .type_size(element)
            .ok_or_else(|| NativeEmitError("array element has no native size".to_owned()))?;
        let size = element_size
            .checked_mul(capacity)
            .ok_or_else(|| NativeEmitError("array layout size overflow".to_owned()))?;
        Ok(ArrayLayout { element, capacity, size, alignment: self.alignment(element)? })
    }
}
