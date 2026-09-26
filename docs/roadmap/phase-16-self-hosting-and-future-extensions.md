# Phase 16: Self-Hosting and Future Language Extensions

These features are intentionally outside the initial Alpha core and require a separate design decision:

- [ ] Explicitly scoped views.
- [x] Single-origin `abs Buffer` views with caller-scope propagation.
- [ ] Zero-copy slices.
- [ ] Borrowed iterators.
- [x] Single-origin `abs` return values for eligible view types.
- [ ] Struct fields containing references.
- [ ] Advanced lifetime relationships.
- [x] Exclusive call-scope mutable borrowing through `ins`.
- [ ] Advanced reborrow chains and shared mutable ownership.
- [ ] Define task-transfer failure ownership for future `act` support.
- [ ] Concurrency and thread-safety semantics.
- [ ] Self-hosting compiler stages.
