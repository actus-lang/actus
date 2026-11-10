# Phase 52: Source Layout Audit

## Status: Proposed

## Scope and invariants

Phase 52 reorganizes `src/` without changing compiler behavior. Directory
roots are facades, implementation files are named for their responsibility,
and every domain family is located once. `X.rs` plus `X/` pairs, generic file
names, parent-name duplicates, and one-file subdirectories are removed.

The public module paths, ABI, diagnostics, snapshots, `actus --version`, and
CLI output remain unchanged. Private source tests move with their module;
integration tests remain under `tests/`.

## Evidence gates

Every PR uses two structural commits where applicable:

1. a `git mv`-only commit whose `git diff -M` reports 100% renames;
2. a module-path/facade commit containing only `mod`, `use`, `pub use`, path,
   and test-module changes.

Before and after each PR, compare `cargo test --all-features -- --list`,
snapshots, `actus --version`, CLI output, `cargo fmt --all -- --check`, and
`git diff --check`. Function bodies are compared byte-for-byte for moves.

## PR sequence

1. Parser, lexer, and AST
2. Target and configuration
3. Semantic
4. Shared lowering, body, and backend contract
5. Cranelift
6. LLVM
7. Runtime, modules, and registry
8. CLI and LSP
9. Formatter, diagnostics, documentation, and conformance
10. Remove the final architecture allowlist

Each module PR performs its complete map, facade conversion, domain grouping,
name corrections, and `tests.rs` relocation once. The architecture gate is
introduced in PR 0 and its exact allowlist shrinks after every PR.

## 1. Parser, lexer, and AST

### Final tree

```text
parser/
  mod.rs
  case.rs conditional.rs contracts.rs cursor.rs declarations.rs enums.rs
  expressions.rs generics.rs grammar.rs packs.rs roles.rs serialization.rs
  spans.rs state.rs statements.rs structs.rs
  metadata/{bounded,capability,declaration,deterministic,
    dispatch,duplicates,effect,fault,limitless,must_use,realtime,repr,
    stack_limit,syntax,target,test}.rs
lexer/
  mod.rs scan.rs token.rs scanner/{mod,state,cursor,identifiers,literals,tokens}.rs
ast/
  mod.rs abi.rs expr.rs intrinsic.rs pattern.rs registry.rs stmt.rs types.rs
  decl/{mod,builtins,callable,data,top_level,types}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `parser/parser.rs` | `parser/state.rs` | rename |
| `parser/mod.rs` `parse` | `parser/grammar.rs` | extract |
| `parser/metadata/*` | `parser/metadata/*` | unchanged |
| `lexer/mod.rs` `scan` | `lexer/scan.rs` | extract |
| `lexer/scanner/mod.rs` `LexErrorKind`, `Scanner` | `lexer/scanner/state.rs` | extract |
| `lexer/scanner/*` | `lexer/scanner/*` | unchanged |
| `ast/decl/*` | `ast/decl/*` | unchanged |
| `ast/{abi,expr,intrinsic,pattern,registry,stmt,types}.rs` | same paths | unchanged |

`Parser` is the primary type in `parser/state.rs`; the parser and lexer roots
now expose the unchanged public paths through `parser/mod.rs` and
`lexer/mod.rs`.

The remaining `#[path]` declarations are explicit temporary exceptions for
later PRs. `src/codegen/body/case.rs` and
`src/codegen/body/case_value.rs` are removed in PR 4; their case lowering
children become ordinary module children. `src/runtime/fs/walk.rs` is removed
in PR 7 when its tests move to `runtime/fs/tests.rs`.
`src/conformance/source_limits.rs` extracts `diagnostics.rs`, `policy.rs`, and
`scan.rs`, while `src/ffi/abi.rs` extracts `abi_errors.rs` and `aggregate.rs`;
both are PR 9 work. `src/diagnostics/renderer/codes/mod.rs` is removed in PR 9
when `pack_codes.rs` becomes the codes facade's regular child.

### PR 1 evidence

- [x] Parser, lexer, and AST layout work is implemented in commits
  [`5866cdc1`](https://github.com/actus-lang/actus/commit/5866cdc1) and
  [`17cdfd44`](https://github.com/actus-lang/actus/commit/17cdfd44); AST files
  remain unchanged as required by this map.
- [x] Architecture allowlist entries for the parser and lexer facades were
  removed in [`ebdd03af`](https://github.com/actus-lang/actus/commit/ebdd03af);
  the no-new-`#[path]` guard and its six exact temporary exceptions were added
  in [`aaebe75b`](https://github.com/actus-lang/actus/commit/aaebe75b).
- [x] `cargo test --all-features -- --list` reports 1,988 tests before and
  after; after excluding Cargo timing lines, the test-name diff is empty.
- [x] `target/debug/actus --version` and
  `target/debug/actus check examples/hello.act --strict` are byte-identical
  before and after the layout change.
- [x] Snapshot files are unchanged; formatting and the architecture test pass.
- [x] The body audit used `git diff 4ff355ca -- src/parser src/lexer` with
  `grep '^[+-]'` filtering. After excluding pure move/extract hunks, the
  remaining semantic changes are limited to `pub(super)` visibility and
  module/import lines; the filtered body-change output is empty.

## 2. Target and configuration

### Final tree

```text
target/{mod,model,contracts,resolution,arm,counter,provider}.rs
configuration/{mod,accessors,checksum,dependencies,loading,lockfile,manifest,
  resolution,strict,types,version}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `target.rs` model items | `target/model.rs` | extract |
| `target.rs` linker/ABI contracts | `target/contracts.rs` | extract |
| `target.rs` target parsing/resolution | `target/resolution.rs` | extract |
| `target.rs` | `target/mod.rs` | pair→facade |
| `target/{arm,counter,provider}.rs` | same paths | unchanged |
| `configuration/*.rs` | same paths | unchanged |
| `configuration/mod.rs` | same path, facade-only | extract |

The target split follows the accepted grouping `target → model/contracts/
resolution`; `arm`, `counter`, and `provider` remain separate domains.

## 3. Semantic

### Final tree

```text
semantic/
  mod.rs target_filter.rs stack_limit.rs
  generics/{mod,types,cache,calls,errors,substitution}.rs
  enums/{mod,model,registry,constructors/{mod,arguments,type_parsing}}.rs
  patterns/{mod,bindings,validation,moves,diagnostics}.rs
  calls/{mod,effects,ownership,parsing,paths,signatures,arguments,shapes}.rs
  analyzer/{mod,analysis,expressions/{mod,conditionals,context,rules},
    statements/{mod,control},bounded/{mod,recursion,graph},constants,
    declarations,for_loops,lifecycle,operator_validation,state,validation,verb}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `semantic/mod.rs` target filtering | `semantic/target_filter.rs` | extract |
| `semantic/mod.rs` stack validation | `semantic/stack_limit.rs` | extract |
| `generics.rs` | `generics/types.rs` | group |
| `generic_cache.rs` | `generics/cache.rs` | group |
| `generic_calls.rs` | `generics/calls.rs` | group |
| `generic_errors.rs` | `generics/errors.rs` | group |
| `type_substitution.rs` | `generics/substitution.rs` | group |
| `enums.rs` | `enums/model.rs` | group |
| `enum_registry.rs` | `enums/registry.rs` | group |
| `enum_constructors/*` | `enums/constructors/*` | group |
| `pattern_moves.rs` | `patterns/moves.rs` | group |
| `pattern_support.rs` | `patterns/diagnostics.rs` | rename |
| `call_arguments.rs` | `calls/arguments.rs` | group |
| `argument_shapes.rs` | `calls/shapes.rs` | group |
| `analyzer/mod.rs` `analyze` | `analyzer/analysis.rs` | extract |
| `analyzer/expression_*.rs` | `analyzer/expressions/*.rs` | group |
| `analyzer/statement_*.rs` | `analyzer/statements/*.rs` | group |
| `analyzer/{bounded,bounded_graph}.rs` | `analyzer/bounded/{recursion,graph}.rs` | group |
| remaining semantic files | same paths | unchanged |

Imports prove the groups: generic calls use `argument_shapes` and `calls`
(`src/semantic/generic_calls.rs:7-8`), enum lowering calls
`enum_constructors::generic_receiver_type` (`src/semantic/enums.rs:33,156`),
pattern validation uses `pattern_support` (`src/semantic/patterns/validation.rs:9`),
and analyzer state stores both call signatures and generic cache
(`src/semantic/analyzer/state.rs:26,64`).

## 4. Shared lowering, body, and contract

### Final tree

```text
codegen/body/{mod,lowering,contract,case/{mod,carried,context,returns,nested,
  value/{mod,values,owner_flow}},loops/{mod,lowering,exits,range_setup,state},control_flow,if_flow,
  provider,statements,try_value,value}.rs
codegen/shared_lowering/expression/{mod,aggregate/{mod,arrays,compound,fields,
  metadata,structs},calls/{mod,arguments,constructors,loan,print,resolution,
  runtime},arena,arithmetic,array_ops,buffer,case,cast,cleanup,compare,dispatch,
  drop,layout,literals,packed,parameter_state,provider,state}.rs
codegen/shared_lowering/layout/{mod,registry,cleanup,generic_struct,options,provider}.rs
codegen/contract/{mod,types,cleanup,dominance,errors,flow,instruction,
  semantic_cleanup,validation}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `body.rs` | `body/mod.rs` | pair→facade |
| `body.rs` `BodyLowering` | `body/lowering.rs` | extract |
| `body_contract.rs` | `body/contract.rs` | move |
| `body/case.rs` | `body/case/mod.rs` | pair→facade |
| `body/case_return.rs` | `body/case/returns.rs` | rename |
| `body/nested_case.rs` | `body/case/nested.rs` | rename |
| `body/case_value.rs` | `body/case/value/mod.rs` | group + extract |
| `body/loop_flow.rs` | `body/loops/exits.rs` | group |
| `body/loop_support.rs` | `body/loops/targets.rs` | rename |
| `body/loops.rs` | `body/loops/mod.rs` | pair→facade |
| `shared_lowering/expression.rs` | `expression/mod.rs` | pair→facade |
| `expression/aggregate.rs` | `expression/aggregate/mod.rs` | pair→facade |
| `expression/calls.rs` | `expression/calls/mod.rs` | pair→facade |
| `shared_lowering/layout.rs` `LayoutRegistry` | `layout/registry.rs` | extract |
| `shared_lowering/support.rs` | `scalar_types.rs`, `integer_values.rs` | rename |
| `contract/mod.rs` backend types | `contract/types.rs` | extract |
| existing contract children | same paths | unchanged |

`body.rs:10-20` imports the case/loop families; aggregate and call modules
are imported through the expression modules. `LayoutRegistry` is the primary
type in `shared_lowering/layout.rs`.

## 5. Cranelift

### Final tree and map

```text
cranelift/numeric/{mod,float.rs,integer.rs}
cranelift/dispatch/{mod,dynamic.rs,vtable.rs}
cranelift/emission.rs and its extracted emission responsibilities
cranelift/calls/{mod,arguments,context,lower,method,returns}.rs
cranelift/case/*  cranelift/contract/*  cranelift/expressions/*
cranelift/function_definition/*  cranelift/layout/*  cranelift/lowering/*
cranelift/performance/*  cranelift/generic/*  cranelift/structs/*
```

| Old | New | Type |
| --- | --- | --- |
| `float_ops.rs` | `numeric/float.rs` | group |
| `integer_ops.rs` | `numeric/integer.rs` | group |
| `dynamic_call.rs` | `dispatch/dynamic.rs` | group |
| `vtable.rs` | `dispatch/vtable.rs` | group |
| `emission.rs` | `emission/output.rs` | group + extract |
| `emitter_declarations.rs` | `emission/externals.rs` | rename |
| `declarations.rs` | `emission/declarations.rs` | group |
| `calls.rs` | `calls/mod.rs` | pair→facade |
| `calls/*` | `calls/*` | unchanged |
| `case/*`, `contract/*`, `expressions/*` | same subtrees | unchanged |
| `function_definition/*`, `layout/*`, `lowering/*`, `performance/*`, `generic/*`, `structs/*` | same subtrees | unchanged |
| remaining `cranelift/*.rs` | same paths | unchanged |

`cranelift` `float_ops`/`integer_ops` share numeric lowering; `dynamic_call`
and `vtable` share dispatch. The similarly named `enum_layout` files remain
backend-specific and are not merged.

## 6. LLVM

### Final tree and exact map

```text
llvm/
  mod.rs arena.rs branches.rs context.rs control_flow.rs
  declarations.rs emission.rs enums.rs failures.rs guards.rs identity.rs
  instruction.rs literals.rs memory.rs module.rs object.rs returns.rs values.rs
  verification.rs calls.rs
  scalar/{mod,values,loan,float}.rs
  scalar/integer/{mod,cast,ops}.rs
  aggregate/{mod,storage,arrays,packed/fields}.rs
  cleanup/{mod,emission,bridge/{mod,address}}.rs
  shim/{mod,options}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `llvm/scalar.rs` | `llvm/scalar/values.rs` | group + extract |
| `llvm/scalar_loan.rs` | `llvm/scalar/loan.rs` | rename |
| `llvm/float.rs` | `llvm/scalar/float.rs` | group |
| `llvm/integer/mod.rs` | `llvm/scalar/integer/mod.rs` | move |
| `llvm/integer/cast.rs` | `llvm/scalar/integer/cast.rs` | move |
| `llvm/integer/ops.rs` | `llvm/scalar/integer/ops.rs` | move |
| `llvm/aggregates.rs` | `llvm/aggregate/storage.rs` | group |
| `llvm/arrays.rs` | `llvm/aggregate/arrays.rs` | group |
| `llvm/packed/mod.rs` | `llvm/aggregate/packed/fields.rs` | extract |
| `llvm/calls.rs` | `llvm/calls.rs` | unchanged |
| `llvm/returns.rs` | `llvm/returns.rs` | unchanged |
| `llvm/cleanup.rs` | `llvm/cleanup/mod.rs` | pair→facade |
| `llvm/cleanup/bridge.rs` | `llvm/cleanup/bridge/mod.rs` | pair→facade |
| `llvm/cleanup/bridge/address.rs` | same path | unchanged |
| `llvm/shim.rs` | `llvm/shim/mod.rs` | pair→facade |
| `llvm/shim/target_options.cpp` | same path | unchanged |
| `llvm/{arena,branches,context,control_flow,declarations,emission,enums,failures,guards,identity,instruction,literals,memory,module,object,values,verification}.rs` | same paths | unchanged |

`aggregates.rs`, `arrays.rs`, and `packed/` are grouped because they are all
aggregate representation/lowering and their use sites cross-reference the
same LLVM aggregate operations. `calls.rs` remains unchanged: it is a single
call implementation file and moving it into a one-file `call/` directory is
explicitly disallowed. `returns.rs` also remains `returns.rs`; the keyword
correction applies only to the proposed `body/case/returns.rs` destination.

## 7. Runtime, modules, and registry

### Final tree and map

```text
runtime/region/{mod,descriptor,access,profile,provider,tests}.rs
runtime/region/bridge/{mod,access,pinning,range,volatile,tests}.rs
runtime/fs/{walk.rs,tests.rs}
modules/aggregation/objects/{mod,plan,types,units}.rs
modules/aggregation/plan/{mod,compilation,validation}.rs
modules/aggregation/visibility/{mod,signature}.rs
```

| Old | New | Type |
| --- | --- | --- |
| `runtime/region.rs` | `region/mod.rs` | pair→facade |
| region descriptor/access items | `region/{descriptor,access}.rs` | extract |
| `region_profile.rs` | `region/profile.rs` | group |
| `region_provider.rs` | `region/provider.rs` | group |
| `region_bridge.rs` | `region/bridge/access.rs` | group + extract |
| `region_pinning_bridge.rs` | `region/bridge/pinning.rs` | group |
| `region_range_bridge.rs` | `region/bridge/range.rs` | group |
| `region_volatile_bridge.rs` | `region/bridge/volatile.rs` | group |
| `region_bridge_tests.rs` | `region/bridge/tests.rs` | rename |
| `region/tests.rs` | same path | unchanged |
| `runtime/fs/walk_tests.rs` | `runtime/fs/tests.rs` | rename |
| `aggregation/object_*.rs` | `aggregation/objects/*.rs` | group |
| `aggregation/plan.rs` | `aggregation/plan/compilation.rs` | group + extract |
| `aggregation/plan_validation.rs` | `aggregation/plan/validation.rs` | group |
| `aggregation/{visibility,signature_visibility}.rs` | `aggregation/visibility/*.rs` | group |
| `registry/{cache,index,trust}.rs` | same paths | unchanged |

`runtime/mod.rs` currently re-exports all region bridge APIs
(`src/runtime/mod.rs:110-120`), proving that they belong to the region domain.

## 8. CLI and LSP

| Old | New | Type |
|---|---|---|
| `cli/build/{emission,llvm_emission,cranelift_shared,shared_emission}.rs` | `cli/build/emission/{mod,llvm,cranelift,shared}.rs` | group |
| `lsp/{protocol,protocol_contract,transport,request_dispatch,server,progress,cancellation}.rs` | `lsp/protocol/{messages,session,transport,dispatch,server,progress,cancellation}.rs` | group |
| `lsp/{query_bounds,query_cache,query_handlers,semantic_model,semantic_query,semantic_tokens,source_queries}.rs` | `lsp/queries/{bounds,cache,handlers,model,semantic,semantic_tokens,source}.rs` | group |
| editor feature files | `lsp/editor/*` | group |
| `lsp/{documents,workspace,uri,position,module_scope}.rs` | `lsp/workspace/{documents,state,uri,position,module_scope}.rs` | group/rename |
| remaining CLI files | same paths | unchanged |

`workspace.rs` does not become `workspace/workspace.rs`; its destination is
`workspace/state.rs`, because it owns workspace state. LSP imports confirm the
protocol/query/editor dependency direction (`src/lsp/server.rs:6-16`,
`src/lsp/query_handlers.rs:5-20`).

## 9. Formatter, diagnostics, documentation, conformance

### Final tree

```text
cli/
  mod.rs build/{mod,emission/{mod,llvm,cranelift,shared},loading,options,
    artifacts,module_type_context,object_manifest}.rs test_runner/{mod,runner,
    collection,execution,output,process}.rs
lsp/
  mod.rs protocol/{mod,messages,session,transport,dispatch,server,progress,
    cancellation}.rs queries/{mod,bounds,cache,handlers,model,semantic,
    semantic_tokens,source}.rs editor/{completion,definition,navigation,rename,
    code_lens,signature_help,formatting,hover}.rs workspace/{documents,state,
    uri,position,module_scope}.rs
formatter/declarations/{mod,render,widths}.rs
diagnostics/renderer/{mod,render,json,module,codes/{mod,packing,extended,mapping,messages}}.rs
documentation/{mod,contract,language,model,rules}.rs
conformance/{mod,diagnostics,exceptions,policy,scan,source_limits,tree_policy}.rs
ffi/{mod,abi,abi_errors,aggregate}.rs
```

`formatter/declarations.rs` and `diagnostics/renderer/mod.rs` receive separate
facade and extraction rows; `diagnostics/strict_codes.rs` remains a single
file at its current path.

| Old | New | Type |
| --- | --- | --- |
| `formatter/declarations.rs` | `formatter/declarations/render.rs` | group + extract |
| `formatter/declaration_widths.rs` | `formatter/declarations/widths.rs` | group |
| `diagnostics/renderer/pack_codes.rs` | `renderer/codes/packing.rs` | rename |
| `diagnostics/renderer/codes/*` | same codes subtree | unchanged |
| `diagnostics/strict_codes.rs` | same path | unchanged |
| documentation files | same paths | unchanged |
| conformance files | same paths | unchanged |
| `conformance/source_limits.rs` | `conformance/{diagnostics,policy,scan}.rs` | extract (`#[path]` removal) |
| `ffi/abi.rs` | `ffi/{abi_errors,aggregate}.rs` | extract (`#[path]` removal) |
| `diagnostics/renderer/codes/mod.rs` | `diagnostics/renderer/codes/packing.rs` | extract (`#[path]` removal) |
| `runtime/fs/walk.rs` | `runtime/fs/walk.rs` | unchanged (`#[path]` removal; child moves with `walk_tests.rs`) |

The two body exceptions are tracked with the codegen map: `body/case.rs` and
`body/case_value.rs` each lose their `#[path]` child declaration in PR 4.

`diagnostics/strict_codes.rs` stays in place because it would otherwise create
a one-file directory. Likewise `llvm/calls.rs` stays at the LLVM root.

## Rejected groups

- `enum_layout.rs` files across AST/Cranelift/LLVM are separate backend layers.
- `generic/layout.rs` is generic layout, not Cranelift machine layout.
- `build_graph.rs` and `packaging.rs` each have no second sibling in the same
  domain and therefore remain standalone files.
- LSP workspace state is not merged with editor features.
- `diagnostics/strict_codes.rs` is not moved into a one-file `codes/` folder.

## Pair conversion details

The `pair→facade` label is used only where the directory already exists. In
each such case the implementation row is separate and names the responsibility
file that receives the existing items:

| Old | Facade | Extracted items | Destination |
| --- | --- | --- | --- |
| `body/case.rs` | `body/case/mod.rs` | case lowering functions | `body/case/branches.rs` |
| `body.rs` | `body/mod.rs` | `BodyLowering` facade exports | `body/lowering.rs` |
| `body/loops.rs` | `body/loops/mod.rs` | loop lowering functions | `body/loops/lowering.rs` |
| `body/case_value.rs` | `body/case/value/mod.rs` | case value lowering functions | `body/case/value/values.rs` |
| `shared_lowering/expression.rs` | `expression/mod.rs` | expression lowering methods | `expression/lowering.rs` |
| `expression/aggregate.rs` | `aggregate/mod.rs` | aggregate lowering functions | `aggregate/values.rs` |
| `expression/calls.rs` | `calls/mod.rs` | call lowering functions | `calls/lowering.rs` |
| `shared_lowering/layout.rs` | `layout/mod.rs` | `LayoutRegistry` and layout helpers | `layout/registry.rs` |
| `llvm/cleanup.rs` | `cleanup/mod.rs` | `emit_cleanup` | `cleanup/emission.rs` |
| `llvm/cleanup/bridge.rs` | `bridge/mod.rs` | bridge argument and symbol functions | `bridge/emission.rs` |
| `llvm/shim.rs` | `shim/mod.rs` | `TargetMachineOptions` and wrapper | `shim/options.rs` |
| `aggregation/visibility.rs` | `visibility/mod.rs` | visibility validation functions | `visibility/checks.rs` |
| `runtime/region.rs` | `region/mod.rs` | region descriptors and access facade | `region/descriptor.rs` |
| `target.rs` | `target/mod.rs` | target model items | `target/model.rs` |
| `target.rs` | `target/mod.rs` | linker and ABI contracts | `target/contracts.rs` |
| `target.rs` | `target/mod.rs` | target parsing and resolution | `target/resolution.rs` |

`body/case/returns.rs` is plural because `return` is a Rust keyword;
`llvm/returns.rs` remains the existing non-keyword filename. The same
facade-plus-extract rule applies to every other true pair in the mapping.

### Group and extract details without an existing pair

| Current file | Primary items | Destination | Type |
| --- | --- | --- | --- |
| `cranelift/emission.rs` | emission entry points | `cranelift/emission/output.rs` | group + extract |
| `llvm/scalar.rs` | scalar selection, guards, constants | `llvm/scalar/values.rs` | group + extract |
| `formatter/declarations.rs` | declaration formatting functions | `formatter/declarations/render.rs` | group + extract |
| `runtime/region_bridge.rs` | region bridge operations | `region/bridge/access.rs` | group + extract |
| `body/loop_flow.rs` | loop exit and flow transitions | `body/loops/exits.rs` | group + extract |
| `modules/aggregation/plan.rs` | `ModuleCompilationPlan` and plan builders | `modules/aggregation/plan/compilation.rs` | group + extract |
| `modules/aggregation/plan_validation.rs` | plan validation functions | `modules/aggregation/plan/validation.rs` | group + extract |

## Exact LLVM map

| Current file | Final path | Type | Reason |
| --- | --- | --- | --- |
| `llvm/aggregates.rs` | `llvm/aggregate/storage.rs` | group | aggregate storage/allocation functions |
| `llvm/arena.rs` | `llvm/arena.rs` | unchanged | arena ownership domain |
| `llvm/arrays.rs` | `llvm/aggregate/arrays.rs` | group | aggregate representation/lowering family |
| `llvm/branches.rs` | `llvm/branches.rs` | unchanged | branch emission domain |
| `llvm/calls.rs` | `llvm/calls.rs` | unchanged | one file; no one-file `call/` directory |
| `llvm/cleanup.rs` | `llvm/cleanup/mod.rs` | pair→facade | existing cleanup directory |
| `llvm/cleanup/bridge.rs` | `llvm/cleanup/bridge/mod.rs` | pair→facade | existing bridge directory |
| `llvm/cleanup/bridge/address.rs` | same path | unchanged | address bridge domain |
| `llvm/context.rs` | `llvm/context.rs` | unchanged | LLVM context ownership |
| `llvm/control_flow.rs` | `llvm/control_flow.rs` | unchanged | control-flow emission |
| `llvm/declarations.rs` | `llvm/declarations.rs` | unchanged | declaration emission |
| `llvm/emission.rs` | `llvm/emission.rs` | unchanged | no sibling directory |
| `llvm/enums.rs` | `llvm/enums.rs` | unchanged | enum emission |
| `llvm/failures.rs` | `llvm/failures.rs` | unchanged | failure values |
| `llvm/float.rs` | `llvm/scalar/float.rs` | group | scalar operations |
| `llvm/guards.rs` | `llvm/guards.rs` | unchanged | guard emission |
| `llvm/identity.rs` | `llvm/identity.rs` | unchanged | toolchain identity |
| `llvm/instruction.rs` | `llvm/instruction.rs` | unchanged | instruction abstraction |
| `llvm/integer/mod.rs` | `llvm/scalar/integer/mod.rs` | group | integer scalar family |
| `llvm/integer/cast.rs` | `llvm/scalar/integer/cast.rs` | group | integer scalar family |
| `llvm/integer/ops.rs` | `llvm/scalar/integer/ops.rs` | group | integer scalar family |
| `llvm/literals.rs` | `llvm/literals.rs` | unchanged | literal emission |
| `llvm/memory.rs` | `llvm/memory.rs` | unchanged | memory operations |
| `llvm/module.rs` | `llvm/module.rs` | unchanged | module construction |
| `llvm/object.rs` | `llvm/object.rs` | unchanged | object emission |
| `llvm/packed/mod.rs` | `llvm/aggregate/packed/fields.rs` | extract | packed field load/store implementation |
| `llvm/returns.rs` | `llvm/returns.rs` | unchanged | single return implementation |
| `llvm/scalar.rs` | `llvm/scalar/values.rs` | extract | no current scalar/ directory; scalar family is grouped by content |
| `llvm/scalar_loan.rs` | `llvm/scalar/loan.rs` | group | scalar loan operations |
| `llvm/shim.rs` | `llvm/shim/mod.rs` | pair→facade | existing C++ shim directory |
| `llvm/shim/target_options.cpp` | same path | unchanged | C++ shim implementation |
| `llvm/values.rs` | `llvm/values.rs` | unchanged | LLVM value wrapper |
| `llvm/verification.rs` | `llvm/verification.rs` | unchanged | IR verification |

## Current allowlist and planned PR

The architecture test has six exact-path allowlists. Their current sizes are:

| List | Count | Current entries move in |
| --- | ---: | --- |
| non-facade `mod.rs` | 32 | PRs 1, 3, 4, 5, 6, 7, 8, 9 |
| X.rs + X/ pairs | 15 | PRs 2, 4, 5, 6, 7 |
| one-file module directories | 7 | PRs 4, 5, 7 |
| parent-name duplicates | 0 | none |
| generic/forbidden filenames | 2 | PRs 3 and 4 |
| `#[path]` module redirections | 6 | PRs 4, 7, and 9 |

### Allowlist entry → PR

| Entry | PR |
| --- | ---: |
| `src/cli/mod.rs` | 8 |
| `src/cli/test_runner/mod.rs` | 8 |
| `src/codegen/mod.rs` | 4 |
| `src/codegen/contract/mod.rs` | 4 |
| `src/codegen/cranelift/case/mod.rs` | 5 |
| `src/codegen/cranelift/contract/mod.rs` | 5 |
| `src/codegen/cranelift/expressions/mod.rs` | 5 |
| `src/codegen/cranelift/function_definition/mod.rs` | 5 |
| `src/codegen/cranelift/layout/mod.rs` | 5 |
| `src/codegen/cranelift/lowering/statements/mod.rs` | 5 |
| `src/codegen/cranelift/lowering/mod.rs` | 5 |
| `src/codegen/cranelift/performance/mod.rs` | 5 |
| `src/codegen/llvm/packed/mod.rs` | 6 |
| `src/codegen/result_constructors/mod.rs` | 4 |
| `src/codegen/symbols/mod.rs` | 4 |
| `src/diagnostics/renderer/mod.rs` | 9 |
| `src/formatter/mod.rs` | 9 |
| `src/lexer/mod.rs` | 1 |
| `src/lexer/scanner/mod.rs` | 1 |
| `src/lsp/hover/mod.rs` | 8 |
| `src/parser/mod.rs` | 1 |
| `src/registry/mod.rs` | 7 |
| `src/runtime/mod.rs` | 7 |
| `src/runtime/fs/mod.rs` | 7 |
| `src/runtime/path/mod.rs` | 7 |
| `src/semantic/analyzer/mod.rs` | 3 |
| `src/semantic/calls/mod.rs` | 3 |
| `src/semantic/enum_constructors/mod.rs` | 3 |
| `src/semantic/errors/mod.rs` | 3 |
| `src/semantic/mod.rs` | 3 |
| `src/semantic/ownership/mod.rs` | 3 |
| `src/semantic/patterns/mod.rs` | 3 |
| `src/modules/aggregation/visibility.rs` | 7 |
| `src/codegen/body.rs` | 4 |
| `src/codegen/body/case.rs` | 4 |
| `src/codegen/body/case_value.rs` | 4 |
| `src/codegen/body/loops.rs` | 4 |
| `src/codegen/cranelift/calls.rs` | 5 |
| `src/codegen/llvm/cleanup.rs` | 6 |
| `src/codegen/llvm/cleanup/bridge.rs` | 6 |
| `src/codegen/llvm/shim.rs` | 6 |
| `src/codegen/shared_lowering/expression.rs` | 4 |
| `src/codegen/shared_lowering/expression/aggregate.rs` | 4 |
| `src/codegen/shared_lowering/expression/calls.rs` | 4 |
| `src/codegen/shared_lowering/layout.rs` | 4 |
| `src/runtime/region.rs` | 7 |
| `src/target.rs` | 2 |
| `src/codegen/cranelift/performance` | 5 |
| `src/codegen/llvm/packed` | 6 |
| `src/codegen/result_constructors` | 4 |
| `src/codegen/symbols` | 4 |
| `src/semantic/enum_constructors` | 3 |
| `src/semantic/errors` | 3 |
| `src/semantic/ownership` | 3 |
| `src/codegen/shared_lowering/support.rs` | 4 |
| `src/semantic/enum_constructors/support.rs` | 3 |
| `src/codegen/body/case.rs` | 4 |
| `src/codegen/body/case_value.rs` | 4 |
| `src/runtime/fs/walk.rs` | 7 |
| `src/conformance/source_limits.rs` | 9 |
| `src/diagnostics/renderer/codes/mod.rs` | 9 |
| `src/ffi/abi.rs` | 9 |

The full exact paths are the constants in
`tests/architecture_conformance.rs`; this table assigns every entry to the
PR that removes it.

### Destination uniqueness audit

The following command extracts old→new pairs from every map table, expands
brace lists, normalizes both sides to `src/` paths using the section context,
and reports only destinations fed by two different old files. Repeating one
old file in several tables is allowed; one old file splitting into several
destinations is also allowed. Its output is intentionally empty:

```sh
python3 - docs/roadmap/phase-52-source-layout.md <<'PY' | sort | uniq -d
import re, sys
from collections import defaultdict
from pathlib import Path

def expand(value):
    match = re.search(r"\{([^{}]+)\}", value)
    if not match:
        return [value]
    prefix, suffix = value[:match.start()], value[match.end():]
    return [item for part in match.group(1).split(',')
            for item in expand(prefix + part + suffix)]

def normalize(path, section):
    path = path.strip()
    if path.startswith('src/'):
        return path
    if path == 'body.rs':
        return 'src/codegen/shared_lowering/body.rs'
    if path.startswith(('body/', 'shared_lowering/', 'contract/')):
        return 'src/codegen/' + path
    if path == 'body.rs' and section == 'shared':
        return 'src/codegen/shared_lowering/' + path
    if path.startswith(('cli/', 'lsp/')):
        return 'src/' + path
    if path.startswith('cranelift/'):
        return 'src/codegen/' + path
    if path.startswith('llvm/'):
        return 'src/codegen/' + path
    if path.startswith('aggregation/'):
        return 'src/modules/' + path
    if path.startswith('runtime/'):
        return 'src/' + path
    if path.startswith('region/'):
        return ('src/runtime/' if section == 'runtime' else 'src/') + path
    roots = {
        'parser': 'src/parser/', 'target': 'src/target/',
        'semantic': 'src/semantic/', 'shared': 'src/codegen/shared_lowering/',
        'cranelift': 'src/codegen/cranelift/', 'llvm': 'src/codegen/llvm/',
        'runtime': 'src/runtime/', 'cli': 'src/cli/',
        'formatter': 'src/', 'diagnostics': 'src/', 'lsp': 'src/',
    }
    if section == 'shared' and path.startswith('expression/'):
        return 'src/codegen/shared_lowering/' + path
    if section == 'shared' and path.startswith('layout/'):
        return 'src/codegen/shared_lowering/' + path
    root = roots.get(section, 'src/')
    return root + path

table = None
section = None
pairs = defaultdict(set)
for line in Path(sys.argv[1]).read_text().splitlines():
    if line.startswith('## '):
        title = line.lower()
        section = ('shared' if 'shared' in title else
                   'cranelift' if 'cranelift' in title else
                   'llvm' if 'llvm' in title else
                   'runtime' if 'runtime' in title or 'modules' in title else
                   'cli' if 'cli' in title or 'lsp' in title else
                   'formatter' if 'formatter' in title or 'diagnostic' in title else
                   'parser' if 'parser' in title else
                   'target' if 'target' in title else
                   'semantic' if 'semantic' in title else None)
    if not line.startswith('|'):
        table = None
        continue
    if not line.startswith('|'):
        table = None
        continue
    cells = [cell.strip() for cell in line.strip('|').split('|')]
    if 'Old' in cells or 'Current file' in cells:
        old = cells.index('Old') if 'Old' in cells else cells.index('Current file')
        new = [i for i, cell in enumerate(cells)
               if cell in {'New', 'Destination', 'Final path'}]
        table = (old, new)
        continue
    if table is None or set(''.join(cells)) <= {'-', ':'}:
        continue
    old_index, new_indexes = table
    old_tokens = [token for token in re.findall(r'`([^`]+)`', cells[old_index])
                  if (token.endswith('.rs') or token.startswith('src/') or
                      '{' in token or '*' in token)]
    new_tokens = [token for index in new_indexes
                  for token in re.findall(r'`([^`]+)`', cells[index])
                  if '*' not in token]
    olds = [normalize(path, section) for token in old_tokens for path in expand(token)]
    news = [normalize(path, section) for token in new_tokens for path in expand(token)]
    if len(olds) == len(news) and len(olds) > 1:
        for old, new in zip(olds, news):
            pairs[new].add(old)
    else:
        for new in news:
            for old in olds:
                pairs[new].add(old)
for new, olds in pairs.items():
    if len(olds) >= 2:
        for _ in olds:
            print(new)
PY
```

```text
(no output)
```

### Non-facade `mod.rs` extraction map

| Current file | Primary items | Destination | PR |
| --- | --- | --- | ---: |
| `src/cli/mod.rs` | `run`, dispatch, command parsing | `cli/dispatch.rs` | 8 |
| `src/cli/test_runner/mod.rs` | `TestOptions`, `test_command` | `cli/test_runner/runner.rs` | 8 |
| `src/codegen/mod.rs` | `LARGE_AGGREGATE_THRESHOLD` | `codegen/constants.rs` | 4 |
| `src/codegen/contract/mod.rs` | `Backend*` types | `codegen/contract/types.rs` | 4 |
| `src/codegen/cranelift/case/mod.rs` | case lowering functions | `cranelift/case/lowering.rs` | 5 |
| `src/codegen/cranelift/contract/mod.rs` | `emit_contract_module_object` | `cranelift/contract/emission.rs` | 5 |
| `src/codegen/cranelift/expressions/mod.rs` | expression lowering functions | `cranelift/expressions/lowering.rs` | 5 |
| `src/codegen/cranelift/function_definition/mod.rs` | `define_function` | `cranelift/function_definition/definition.rs` | 5 |
| `src/codegen/cranelift/layout/mod.rs` | `LayoutRegistry`, layout types | `cranelift/layout/registry.rs` | 5 |
| `src/codegen/cranelift/lowering/statements/mod.rs` | `lower_statements` | `cranelift/lowering/statements/lowering.rs` | 5 |
| `src/codegen/cranelift/performance/mod.rs` | performance registry/emission | `cranelift/performance/registry.rs` | 5 |
| `src/codegen/llvm/packed/mod.rs` | packed field load/store | `llvm/aggregate/packed/fields.rs` | 6 |
| `src/codegen/result_constructors/mod.rs` | `normalize_program` | `codegen/result_constructors/normalize.rs` | 4 |
| `src/codegen/symbols/mod.rs` | `SymbolKind`, `SymbolIdentity`, registry | `codegen/symbols/model.rs` | 4 |
| `src/diagnostics/renderer/mod.rs` | renderer and diagnostic functions | `diagnostics/renderer/render.rs` | 9 |
| `src/formatter/mod.rs` | `Formatter`, program formatting | `formatter/program.rs` | 9 |
| `src/lexer/mod.rs` | `scan` | `lexer/scan.rs` | 1 |
| `src/lexer/scanner/mod.rs` | `LexErrorKind`, `Scanner` | `lexer/scanner/state.rs` | 1 |
| `src/lsp/hover/mod.rs` | `HoverInfo`, hover functions | `lsp/hover/model.rs` and `hover/render.rs` | 8 |
| `src/parser/mod.rs` | `parse` | `parser/grammar.rs` | 1 |
| `src/registry/mod.rs` | `RegistryError`, `LocalRegistry` | `registry/local.rs` | 7 |
| `src/runtime/mod.rs` | `runtime_archive_path` | `runtime/archive.rs` | 7 |
| `src/runtime/fs/mod.rs` | filesystem mode constants and path helpers | `runtime/fs/mode.rs` | 7 |
| `src/runtime/path/mod.rs` | path model and separators | `runtime/path/model.rs` | 7 |
| `src/semantic/analyzer/mod.rs` | `analyze`, stack validators | `semantic/analyzer/analysis.rs` | 3 |
| `src/semantic/calls/mod.rs` | `Analyzer` call validation impls | `semantic/calls/validation.rs` | 3 |
| `src/semantic/enum_constructors/mod.rs` | constructor validation impls | `semantic/enums/constructors/validation.rs` | 3 |
| `src/semantic/errors/mod.rs` | `SemanticError` | `semantic/errors/model.rs` | 3 |
| `src/semantic/mod.rs` | target filtering and stack-limit validation | `semantic/target_filter.rs`, `semantic/stack_limit.rs` | 3 |
| `src/semantic/ownership/mod.rs` | ownership `Analyzer` impls | `semantic/ownership/validation.rs` | 3 |
| `src/semantic/patterns/mod.rs` | pattern `Analyzer` impls | `semantic/patterns/validation.rs` | 3 |

## Architecture protection

`tests/architecture_conformance.rs` records the current violations by exact
path and rejects any new violation. The allowlist is removed progressively as
PRs 1–9 land.
