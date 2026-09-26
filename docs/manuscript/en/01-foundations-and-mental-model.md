# Chapter 01: Foundations & The Ergative Mental Model

Actus is a systems programming language for writing software whose behavior
must remain understandable all the way down to memory: compilers, embedded
firmware, robotics software, operating-system components, and other programs
where an accidental allocation, an invalid pointer, or an unpredictable
cleanup path can become a real failure.

This chapter builds the mental model used throughout the Manuscript. We begin
with the physical problem, introduce Actus's linguistic idea, and learn the
three words that describe participation in an action: `erg`, `abs`, and `dat`.

The examples are small so we can see what the compiler proves and what the
machine does.

## 1. The Core Problem: Why Systems Programming Is Hard

A running program is not an abstract cloud of values. It is a sequence of
machine instructions operating on a finite set of physical resources.

The CPU executes instructions and keeps a small amount of immediately
available data in registers. Memory outside those registers is normally
organized into regions with different responsibilities. The **stack** is a
fast, automatically scoped area used for call frames and local values. When a
function is called, the machine reserves a frame; when the function returns,
that frame is released. The **heap** is a separately managed region used for
data whose size or lifetime cannot be tied directly to one call frame.

For example, a simplified call stack might look like this:

```text
Higher addresses
┌────────────────────────────────┐
│ caller frame                   │
├────────────────────────────────┤
│ return address                 │
│ count = 3                      │
│ buffer_ptr = 0x1000            │
└────────────────────────────────┘
                 │ points to
                 ▼
Heap             ┌────────────────┐
                 │ buffer bytes   │
                 └────────────────┘
Lower addresses
```

The pointer stored in `buffer_ptr` is only an address. It does not prove that
the bytes are still allocated, correctly typed, or not already released.
Those proofs are the difficult part of systems programming.

### C: freedom with a manual contract

C exposes the machine directly. A programmer can request heap memory with
`malloc` and return it with `free`:

```c
int *number = malloc(sizeof(int));
*number = 42;
free(number);
```

The type `int *` is an address for an integer. `malloc` reserves bytes and
returns an address; `*number = 42` writes through it; `free(number)` returns
the allocation to the heap. C does not prove that the address is used exactly
once or only while valid.

That freedom permits serious bugs. A **dangling pointer** survives its
allocation and causes a use-after-free when read. A **double free** releases
one allocation twice and can corrupt allocator bookkeeping. A **memory leak**
loses the address before `free`, leaving bytes unavailable. The visible failure
may appear far from the cause.

### Garbage collection: a different trade-off

Languages such as Java, Python, and Go reduce the burden of manually releasing
many objects by using a **garbage collector**. The runtime periodically finds
objects that can no longer be reached and reclaims their memory. This adds a
runtime policy that a program does not directly control.

Collection work can introduce pauses and background CPU activity, while the
runtime needs metadata and reserved memory. That matters on a small
microcontroller and does not provide every device driver's deterministic
ownership boundary.

### Rust: compile-time ownership with a demanding vocabulary

Rust rejects many memory errors before execution. Its ownership and borrow
checker make aliasing and lifetime relationships explicit without a tracing
collector.

Rust requires substantial reasoning at API boundaries:
shared and mutable borrows, reborrowing, lifetime parameters such as `'a`,
generic bounds, and relationships between several references. This vocabulary
can obscure the simpler API question: “Am I reading this value, changing it,
or giving it away?”

Actus communicates that answer through a small set of grammatical roles while
keeping ownership deterministic and free from tracing GC.

## 2. The Linguistic Metaphor: What “Ergative” Means

The word **ergative** comes from linguistics. It describes a way of organizing
sentences around the role an entity plays in an action. You do not need a
linguistics background to use the idea.

In a familiar nominative-accusative sentence, we might ask:

```text
The programmer (who) opened the file (what).
```

The grammar emphasizes the subject that performs the action and the object
that receives it. An ergative description instead makes the relationship to
the action itself central: is this participant the one acting, the one being
observed, or the destination receiving a result?

Actus applies that question to computation. Every function-like operation is
a `verb`. A value is not merely “there”; it participates in the verb under an
explicit contract. The compiler checks that contract in much the same way a
grammar checker checks whether the participants in a sentence have compatible
roles.

This metaphor makes ownership precise: instead of inferring it from a function
name or comment, the reader can inspect each parameter's role in the action.

## 3. The Three Foundational Roles

Actus begins with three roles. They describe capabilities and ownership
transitions; they are not runtime objects and do not create wrapper values.

### `erg`: the exclusive owner and mutator

`erg` identifies the active agent of an operation: the binding that owns a
resource and has the authority to modify it. At a declaration boundary,
`erg` means that the receiving operation gets exclusive access for the duration
of that call. At a local binding, it means that the binding is responsible for
the value while it remains live.

Consider a resource represented by a pointer and a length:

```text
Stack frame
┌────────────────────────────────┐
│ erg packet                     │
│ data_ptr = 0x1000              │
│ length = 1024                  │
└────────────────────────────────┘
                 │ owns
                 ▼
Heap/device region
┌────────────────────────────────┐
│ packet bytes                   │
└────────────────────────────────┘
```

The stack binding is small, but represents responsibility for the bytes. When
it leaves its block, the compiler arranges the resource's cleanup on every
valid exit path. The operation depends on the type's cleanup contract, but is
planned statically rather than discovered later by a collector.

### `abs`: the immutable inspector

`abs` identifies read-only inspection. An `abs` argument may read the value but
may not mutate it, take ownership of it, or destroy it. The owner remains
active during inspection.

An inspection does not duplicate or transfer the resource:

```text
Caller frame                 Callee frame
┌──────────────────┐         ┌──────────────────┐
│ erg packet       │         │ abs view         │
│ pointer: 0x1000  │───────> │ pointer: 0x1000  │
└──────────────────┘         └──────────────────┘
                                      │ reads only
                                      ▼
                              ┌──────────────────┐
                              │ packet bytes     │
                              └──────────────────┘
```

The callee's view is temporary. It cannot outlive the owner or become an owned
resource by being returned. When the call ends, the view disappears and the
owner remains live.

### `dat`: the consumer and terminal receiver

`dat` identifies the destination of an ownership transfer. The simplest
physical analogy is handing someone a tool: while the tool is in your hand,
you can use it; after you hand it over, you no longer have authority to use or
destroy it. The recipient now has that responsibility.

If a caller passes an owned resource to a `dat` parameter, the compiler marks
the caller's binding as moved at the call boundary. Any later attempt to read,
mutate, move, or drop that binding is rejected. The receiving operation becomes
the only ownership path and its scope eventually performs the cleanup.

This is not a copy. Copying would create a second independent value. A `dat`
transfer changes who is responsible for the same value.

### The complete lifecycle

The three roles form a readable lifecycle:

```text
1. Creation                 2. Inspection
┌──────────────────────┐    ┌──────────────────────┐
│ erg resource         │    │ abs resource         │
│ owner: caller        │───>│ owner: caller        │
│ Active / Mutable     │    │ Frozen / Read-only   │
└──────────────────────┘    └──────────────────────┘
                                      │ view ends
                                      ▼
3. Handoff               4. Scope exit
┌──────────────────────┐    ┌──────────────────────┐
│ dat resource         │───>│ consumer owns value  │
│ caller: Moved        │    │ cleanup: exactly once│
└──────────────────────┘    └──────────────────────┘
```

An `abs` view temporarily changes access, not ownership. A `dat` transfer
changes ownership and invalidates the source binding.

## 4. Dissecting Your First Actus Programs

### 4.1 The simplest valid entrypoint

```actus
verb main() -> Int {
    return 0;
}
```

Read the declaration from left to right. `verb` is Actus's keyword for an
executable operation. It emphasizes that a declaration describes an action,
rather than borrowing terminology from another language.

`main` is the declaration's identifier. The empty parentheses `()` say that
the operation accepts no parameters. The arrow `->` separates the parameter
list from the return type, and `Int` says that the operation returns an
integer. The opening `{` starts the body; the closing `}` ends it.

Inside the body, `return` ends the operation and supplies its result. The
literal `0` is an integer and the semicolon terminates the statement. For an
executable target, the entry contract maps this integer to process exit status
`0`. The compiler checks compatibility with `Int`; native code places the
result in the target's return-value location.

### 4.2 A real ownership pipeline

The next example contrasts inspection with consumption:

```actus
verb inspect(abs input: Int) -> Int {
    return input;
}

verb consume(dat input: Int) -> Int {
    return input;
}

verb main() -> Int {
    erg value = 42;
    erg seen = inspect(input: value);
    erg final = consume(input: value);
    return final;
}
```

In `verb inspect(abs input: Int) -> Int {`, `verb` begins an operation named
`inspect`. The parameter name is `input`; the role `abs` says that this call
borrows read-only access, and `Int` gives the parameter its type. The return
arrow and `Int` require an integer result. The body returns `input`, which is
legal because reading is allowed.

The `return input;` line evaluates the parameter and ends the callee. Because
the parameter is `abs`, the caller's ownership is not consumed.

The second declaration is similar, but `dat` changes the contract: `consume`
receives ownership. Returning `input` transfers that value onward, so the
callee has no remaining owned binding to clean up for it.

Now examine `main` line by line. `verb main() -> Int {` declares the entry
operation. `erg value = 42;` creates an owned local: `value` is its name, `=`
initializes it, and `42` is the integer. For this scalar, the stack may contain
the value directly:

```text
main frame after initialization
┌─────────────────────────┐
│ value: 42  (Active)     │
└─────────────────────────┘
```

`erg seen = inspect(input: value);` creates another owned local named `seen`.
The label `input:` selects `inspect`'s parameter, and `value` is passed as an
`abs` view. During the call:

```text
caller frame                 callee frame
┌──────────────────────┐     ┌──────────────────────┐
│ value: 42  Active   │─────>│ input: 42  abs       │
│ seen: uninitialized │      └──────────────────────┘
└──────────────────────┘
```

When `inspect` returns, the view ends. `value` remains active and `seen`
receives `42`.

`erg final = consume(input: value);` is the ownership boundary. The argument
label again identifies `input`, but this time the parameter is `dat`. The
compiler marks `value` as moved at this exact call:

```text
caller frame after the call
┌────────────────────────────┐
│ value: Moved               │
│ seen: 42                   │
│ final: 42                  │
└────────────────────────────┘
No further use of value is legal.
```

The machine may copy an `Int` in a register because it has no resource
destructor. That does not change the semantic rule: ownership moved to
`consume` and then to `final`.

Finally, `return final;` returns the result to the entry boundary. If we added
`inspect(input: value);` after the `consume` call, the compiler would reject
the program. `value` is already moved, so using it would be a use-after-move.
The rejection happens before native code generation; there is no runtime check
and no opportunity for undefined behavior.

## 5. Under the Hood: What the Compiler Actually Does

Actus follows a one-directional compilation pipeline. First, the **lexer**
reads characters and groups them into tokens: keywords such as `verb`, `erg`,
`abs`, and `dat`; identifiers such as `main`; literals such as `42`; and
punctuation such as `(`, `)`, `{`, `}`, `:`, `;`, and `->`.

The **parser** consumes those tokens and constructs an **abstract syntax tree
(AST)**. The AST is a tree because a declaration contains a parameter list and
a body, while a body contains statements and expressions. Source spans are
retained so that later diagnostics can point back to the original characters.

The **semantic analyzer** asks questions syntax alone cannot answer: does
`inspect` exist, is `input` an `Int`, does the role correspond to the argument, and is
`value` still available? It tracks uninitialized, active, moved, partially
moved, and dropped ownership states, while access state distinguishes mutable
access from temporary frozen inspection. It validates exits and builds a
deterministic cleanup plan.

Only a semantically valid program reaches **Cranelift IR**. Cranelift uses an
SSA-style intermediate representation: a computed value is assigned a stable
definition, and control-flow joins make its inputs explicit. Actus lowers
validated expressions, calls, layouts, and cleanup actions into that IR.
Cranelift then emits native object code and the selected linker produces the
target executable.

```text
Actus source
    │
    ▼
Lexer ── tokens ──▶ Parser ── AST ──▶ Semantic analysis
                                      │
                         role and ownership invariants
                                      │
                                      ▼
                              Cranelift SSA IR
                                      │
                                      ▼
                          native machine instructions
```

There is no tracing collector, background memory-scanning thread, or mandatory
allocator merely to run an integer-returning function. For an owned resource,
the compiler selects its cleanup operation and schedules it. A transfer
invalidates the old ownership path and assigns cleanup to the new one.

## 6. The `no_std` Architecture Philosophy

To an everyday programmer, `no_std` means that basic execution does not require
a desktop operating system or large standard library. A bare-metal
microcontroller may have a CPU, RAM, flash, timers, and peripherals, but no
process loader, filesystem, terminal, or operating-system allocator.

An Actus program can keep its core semantics independent of those services.
The language does not require a garbage collector to decide when values die,
and a target can provide only the runtime and hardware operations it needs. A
robotics controller can use bounded buffers and fixed layouts; an edge device
can reserve predictable memory; firmware need not import desktop services for
ordinary arithmetic.

The reference bootstrap compiler is written in Rust and targets Cranelift.
Bare-metal work must define startup, runtime facilities, linker contracts,
allocation policy, and device backends. The ownership model does not depend on
tracing GC or an implicit heavyweight runtime.

## 7. Summary: A Mental Checklist

When reading Actus, ask three questions:

1. **What action is taking place?** The operation is a `verb`, and its
   signature describes the participants.
2. **What authority does each value have?** `erg` owns and may mutate, `abs`
   inspects without consuming, and `dat` receives ownership.
3. **Who cleans up, and when?** The compiler proves the ownership path and
   plans deterministic cleanup before generating native code.

In Chapter 02, we will turn this mental model into a real project. We will
install the compiler, create an Arca project, inspect its source layout, run
`verb main() -> Int`, and build a native executable from the command line.
