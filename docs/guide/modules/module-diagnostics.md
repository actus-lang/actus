# Module diagnostics

Common module errors include:

- a directory without its canonical facade;
- a facade that names a missing sibling;
- an import that bypasses a parent facade;
- an unknown or private declaration in an external signature;
- duplicate declarations or exports;
- a source root or package path that does not match the manifest.

Start with the first reported module path and verify the filesystem shape. Then
check the canonical facade and its exports. The compiler should report the
module boundary directly; do not solve the error by changing ownership roles,
copying declarations, or making unrelated symbols public.
