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
            _ => return Ok(None),
        };
        Ok(Some(self.cleanup_descriptor(bridge, root_layout, Some(field))))
    }
}
