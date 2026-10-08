# Arenas and bounded placement

An `Arena[N]` is a bounded storage region for values whose lifetime belongs to
one arena owner. It is useful when a program needs several related values to
live together, such as nodes in a tree or a graph. The arena gives the program
one explicit lifetime boundary instead of requiring a separate general-purpose
heap for every placed value.

`N` is a compile-time capacity for the arena storage. It is not the number of
objects that may be placed. Each placement consumes space according to the
value's native size and alignment, so two small values may fit where two larger
values do not.

## Creating an arena and placing a value

Create an arena with a fixed capacity and place a value into it:

```actus
struct Node {
    value: Int,
}

verb main() -> Int {
    erg arena: Arena[64] = Arena[64]();
    erg first: Node = arena.place(value: Node { value: 20, });
    erg second: Node = arena.place(value: Node { value: 22, });
    return first.value + second.value;
}
```

`arena` owns the bounded storage. `place` constructs the value inside that
storage and returns an arena-backed value that can be used through normal
field access. Placement does not make the arena an unbounded collection and
does not turn the program into a garbage-collected runtime.

A placement can also store a fixed aggregate:

```actus
verb array_example() -> Int {
    erg arena: Arena[256] = Arena[256]();
    erg values: Array[Int, 2] = arena.place(
        value: Array[Int, 2](),
    );
    values[1] = 7;
    return values[1];
}
```

The element must have a known native layout. Unsized or otherwise unsupported
values cannot be placed into a bounded arena because the compiler would not
know how much storage or alignment the placement requires.

## References and recursive data

A common arena use is a recursive value whose child links are optional
references to the same value type. The reference is represented in the type,
so the compiler can check the relationship between the node and the arena that
owns it:

```actus
struct ListNode {
    value: Int,
    next: Option[abs ListNode],
}

verb sum(abs node: ListNode) -> Int {
    abs next = ref node.next;
    return node.value + case next {
        Option.Some(child) => sum(node: abs child),
        Option.None => 0,
    };
}

verb main() -> Int {
    erg arena: Arena[256] = Arena[256]();
    erg tail = arena.place(value: ListNode {
        value: 30,
        next: Option[abs ListNode].None,
    });
    erg head = arena.place(value: ListNode {
        value: 10,
        next: Option[abs ListNode].Some(tail),
    });
    abs root = ref head;
    return sum(node: abs root);
}
```

`Option[abs ListNode]` means that a link may be absent or may refer to another
node. `ref` creates a view for inspection. The view is tied to the lifetime of
the arena-backed value; it is not an independent owner.

The same pattern supports trees and graphs. For a tree, a node can contain two
optional child references:

```actus
struct TreeNode {
    value: Int,
    abs left: Option[abs TreeNode],
    abs right: Option[abs TreeNode],
}
```

Use the ownership rules for [shared views](../ownership/abs.md),
[borrowing and loans](../ownership/borrowing-and-loans.md), and
[cleanup](../ownership/cleanup-and-scope.md) when traversing these values.

## Lifetime and cleanup

The arena owner controls the lifetime of every value placed into it. When the
owner leaves scope, its storage is cleaned up as one bounded operation. A live
view cannot outlive that owner:

```actus
verb invalid_lifetime() -> Int {
    erg arena: Arena[128] = Arena[128]();
    erg node = arena.place(value: Node { value: 1, });
    abs view = ref node;
    drop(arena);
    return view.value;
}
```

The compiler rejects this program because `view` is still live when `arena` is
explicitly dropped. The same rule applies at an ordinary scope boundary. Move
or end the view before destroying the arena; do not copy a reference into a
separate owner to bypass the lifetime check.

Arena cleanup is deterministic and follows Actus scope cleanup. It does not
scan the program for unreachable objects and does not reclaim individual nodes
through a tracing collector. If an application needs independent lifetimes,
use separate owners or an API whose contract provides that lifetime model.

## Capacity and failure

Placement consumes aligned space. When the next value cannot fit, the bounded
capacity contract is violated. Current native execution terminates through a
deterministic native trap rather than silently growing the arena or returning an
untracked allocation.

Choose `N` from the largest supported placement set, including alignment and
nested aggregate size. A successful semantic check proves that the type is
placeable; it does not prove that a particular runtime sequence will fit in
its chosen capacity.

## What an arena is not

An arena is not:

- a general-purpose heap;
- an unbounded dynamic collection;
- a garbage collector;
- a way to make a reference independent of its owner;
- permission to use raw operating-system pointers in Actus values.

It is a fixed, compiler-checked storage boundary with deterministic cleanup.
Keep the arena size, placed value layout, and reference lifetime visible in the
API that uses it.

## Related reference material

- [Structs](structs.md) explains aggregate fields and construction.
- [Arrays and buffers](arrays-and-buffers.md) explains bounded inline storage.
- [Generics](generics.md) explains fixed compile-time parameters.
- [Ownership roles](../ownership/roles.md) explains `erg`, `abs`, `dat`, and
  `ins`.
- [Arena semantic tests](../../../tests/semantic/arenas.rs) and [native arena
  tests](../../../tests/arenas_cli.rs) are the executable language references.
