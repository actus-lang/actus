# Borrowing and loans

Borrowing temporarily changes how an owner may be used. `abs` creates a
read-only view; `ins` creates an exclusive mutable loan.

A view or loan has a scope. The owner becomes available again only after the
borrow ends. The compiler tracks the origin of a returned view and prevents it
from escaping the lifetime of its source.

For indexed storage, a loan refers to the selected place while the original
array or pack owner remains suspended for the call:

```actus
verb update_slot(ins slot: Cell) -> Void {
    slot.value += 1u8;
}

verb update_cell(erg cells: Array[Cell, 4], abs index: u32) -> Void {
    update_slot(slot: ins cells[index]);
}
```

The exact element type and declaration must satisfy the indexed-place contract.
The compiler rejects aliasing loans, loans from read-only owners, loan escape,
and mutation while an incompatible view remains active.
