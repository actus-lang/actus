# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 11. Conditions, loops, and assignment

### 11.1 If statements

```act
if index >= count {
    break;
}
```

The condition must be `Bool`. Branches are normal blocks and preserve
ownership/cleanup semantics.

### 11.2 If expressions

```act
erg selected: u32 = if ready {
    41u32
} else {
    7u32
};
```

Both value-producing branches must unify to a compatible type. A diverging
branch may be used where the other branch determines the expression type.
Nested and `else if` forms must be tested through parser, semantic, and native
execution layers.

### 11.3 Loops

```act
loop {
    if done {
        break;
    }
    continue;
}
```

`break` and `continue` target the nearest enclosing loop. They unwind local
owned values correctly. Loop-carried state must be initialized and updated in
an ownership-safe way.

Actus also supports two bounded `for` forms:

```act
for erg index: u32 in 0u32 .. 8u32 {
    total += index;
}

for erg index: u32 in values {
    total += values[index];
}
```

The range is half-open: `start` is included and `end` is excluded. Both range
bounds must have the declared primitive integer type of the `erg` index
binding. Empty and reversed ranges execute zero times. Values that overflow
the declared bound type are rejected during semantic analysis.

The collection form is index iteration over a statically known `Array[T, N]`.
It also supports pack fields backed by fixed arrays. The compiler derives the
finite bound from the array layout; the loop does not discover a runtime
length. The binding is an explicit `erg` integer index. `abs`, `ins`, and `dat`
bindings are parsed but rejected by semantic analysis in the current profile.

Dynamic collections, scalar sources, unbounded iterators, runtime length
discovery, and hidden collection allocation are not valid `for` sources. Native
lowering emits integer condition, body, increment, and exit blocks. `continue`
always targets the increment block, so it cannot skip index advancement. The
existing cleanup plan applies to `break`, `continue`, nested loops, and early
return. Formatter, definition lookup, and hover preserve and expose the loop
binding like other local bindings.

`repeat erg index: u32 in start .. end { ... }` is a readability helper for the
same bounded range contract. The compiler expands it to canonical `for` form
before semantic analysis; formatter output uses the canonical `for` spelling.
It does not add callbacks, allocation, dynamic bounds, or a second ownership
model.

### 11.4 Assignment

Simple assignment:

```act
counter = 1u32;
packet[index] = byte;
state.flags = state.flags | 0x01u8;
```

Compound assignment is supported for scalar, field, index, and pack-field
places where the operator and types are valid:

```act
index += 1u32;
state.flags |= 0x01u8;
packet[offset] ^= mask;
```

The left-hand place must be evaluated exactly once. Native lowering must
calculate an address, load, operate, and store through that same place. Never
rewrite a compound assignment into a duplicated side-effecting index or field
calculation by hand.
