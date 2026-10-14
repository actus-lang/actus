impl BackendCleanupProvider for ProgramLayouts<'_> {
    fn resolve_binding(
        &self,
        type_name: Option<&TypeName>,
        builtin: Option<BuiltinType>,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        if let Some(type_name) = type_name
            && !matches!(type_name.name.as_str(), "Buffer" | "Region")
            && self
                .type_layout(type_name, &mut Vec::new())
                .is_some_and(|layout| layout.size >= crate::codegen::LARGE_AGGREGATE_THRESHOLD)
        {
            let layout = self
                .type_layout(type_name, &mut Vec::new())
                .ok_or(BackendContractError::InvalidCleanupOperand)?;
            return Ok(Some(self.cleanup_descriptor(
                BackendCleanupBridge::AggregateDrop {
                    size: layout.size,
                    alignment: layout.alignment,
                },
                layout,
                None,
            )));
        }
        if builtin == Some(BuiltinType::Buffer)
            || type_name.is_some_and(|name| name.name == "Buffer")
        {
            return Ok(Some(self.cleanup_descriptor(
                BackendCleanupBridge::BufferDrop,
                self.pointer_layout(),
                None,
            )));
        }
        if type_name.is_some_and(|name| name.name == "Region") {
            return Ok(Some(self.cleanup_descriptor(
                BackendCleanupBridge::RegionDrop,
                self.pointer_layout(),
                None,
            )));
        }
        if let Some(type_name) = type_name
            && let Some(symbol) = self.drop_symbols.get(&type_name.canonical_key())
        {
            let Some(layout) = self.type_layout(type_name, &mut Vec::new()) else {
                return Ok(None);
            };
            return Ok(Some(BackendCleanupResolvedDescriptor {
                descriptor: BackendCleanupBridgeDescriptor {
                    bridge: BackendCleanupBridge::PerformanceDrop,
                    argument: BackendCleanupArgument::Address,
                    result: BackendCleanupResult::Ignored,
                },
                layout,
                field: None,
                performance_symbol: Some(symbol.clone()),
            }));
        }
        Ok(None)
    }

    fn resolve_field(
        &self,
        root_type: &TypeName,
        path: &str,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        let Some(root_layout) = self.type_layout(root_type, &mut Vec::new()) else {
            return Ok(None);
        };
        let Some((field_type, field)) = self.cleanup_field(root_type, path) else {
            return Ok(None);
        };
        let bridge = match field_type.name.as_str() {
            "Buffer" => BackendCleanupBridge::BufferDrop,
            "Region" => BackendCleanupBridge::RegionDrop,
            _ if self.drop_symbols.contains_key(&field_type.canonical_key()) => {
                return Ok(Some(BackendCleanupResolvedDescriptor {
                    descriptor: BackendCleanupBridgeDescriptor {
                        bridge: BackendCleanupBridge::PerformanceDrop,
                        argument: BackendCleanupArgument::FieldAddress,
                        result: BackendCleanupResult::Ignored,
                    },
                    layout: root_layout,
                    field: Some(field),
                    performance_symbol: self.drop_symbols.get(&field_type.canonical_key()).cloned(),
                }));
            }
            _ => {
                let Some(layout) = self.type_layout(&field_type, &mut Vec::new()) else {
                    return Ok(None);
                };
                if layout.size < crate::codegen::LARGE_AGGREGATE_THRESHOLD {
                    return Ok(None);
                }
                return Ok(Some(self.cleanup_descriptor(
                    BackendCleanupBridge::AggregateDrop {
                        size: layout.size,
                        alignment: layout.alignment,
                    },
                    root_layout,
                    Some(field),
                )));
            }
        };
        Ok(Some(self.cleanup_descriptor(bridge, root_layout, Some(field))))
    }

    fn resolve_partial_binding(
        &self,
        root_type: &TypeName,
        moved_fields: &[String],
    ) -> Result<Vec<BackendCleanupResolvedDescriptor>, BackendContractError> {
        let mut paths = Vec::new();
        self.collect_cleanup_paths(root_type, "", moved_fields, &mut paths);
        paths
            .into_iter()
            .filter_map(|path| match self.resolve_field(root_type, &path) {
                Ok(Some(descriptor)) => Some(Ok(descriptor)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }
}

impl ProgramLayouts<'_> {
    fn collect_cleanup_paths(
        &self,
        type_name: &TypeName,
        prefix: &str,
        moved_fields: &[String],
        paths: &mut Vec<String>,
    ) {
        let Some(definition) = self.structs.get(type_name.name.as_str()) else { return };
        for field in &definition.fields {
            if matches!(field.role, crate::ast::StructFieldRole::Abs | crate::ast::StructFieldRole::Ins)
            {
                continue;
            }
            let path = if prefix.is_empty() {
                field.name.clone()
            } else {
                format!("{prefix}.{}", field.name)
            };
            if moved_fields.iter().any(|moved| moved == &path) {
                continue;
            }
            let has_nested_move = moved_fields
                .iter()
                .any(|moved| moved.starts_with(&format!("{path}.")));
            if has_nested_move {
                self.collect_cleanup_paths(&field.ty, &path, moved_fields, paths);
                continue;
            }
            if self.field_requires_cleanup(&field.ty) {
                paths.push(path);
            } else {
                self.collect_cleanup_paths(&field.ty, &path, &[], paths);
            }
        }
    }

    fn field_requires_cleanup(&self, type_name: &TypeName) -> bool {
        matches!(type_name.name.as_str(), "Buffer" | "Region")
            || self.drop_symbols.contains_key(&type_name.canonical_key())
            || self
                .type_layout(type_name, &mut Vec::new())
                .is_some_and(|layout| layout.size >= crate::codegen::LARGE_AGGREGATE_THRESHOLD)
    }
}
