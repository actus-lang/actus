# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 8. Verb declarations and calls

### 8.1 Ordinary verbs

```act
verb add(erg left: Int, erg right: Int) -> Int {
    return left + right;
}
```

Parameters have a role, a name, and a type. The return type follows `->`.
Named call arguments are preferred because they make ownership explicit:

```act
erg total = add(left: 2, right: 3);
```

### 8.2 Visibility

`open` makes a declaration eligible to cross a module facade:

```act
open verb public_checksum(abs packet: Buffer) -> Int {
    return checksum(packet: abs packet);
}
```

An unmarked declaration is private to its source/module boundary. `open` does
not mean “global”; the canonical facade still has to re-export a sibling
declaration.

### 8.3 Method-like calls

The compiler supports method-like calls for role/performance and aggregate
operations where the receiver contract exists. The current language also
allows ordinary verbs with a named receiver parameter. Do not invent a
separate `self` keyword; `self` is not a special receiver keyword.

### 8.4 Static and dynamic dispatch

Static dispatch is the default. A parameter may specify dynamic role dispatch
where a role contract supports it:

```act
verb write_all[Target: Writer](ins target: Target, abs bytes: Buffer) -> Result[Int, IoError] {
    return write(target: ins target, buffer: abs bytes);
}
```

The exact runtime dispatch form must follow existing role/perform and standard
library examples. Dynamic dispatch is an explicit ABI/runtime boundary, not a
reason to put backend-specific vtables in the AST.
