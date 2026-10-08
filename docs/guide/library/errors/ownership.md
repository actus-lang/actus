# Error ownership

An error enum is still an Actus value with ownership state. When a result is
matched with `case dat`, its payload is consumed by the branch. Return it,
translate it, or otherwise finish its ownership path before leaving the
branch.

Runtime bridges may use signed status integers internally. The public facade
must translate those statuses into the module's typed error enum. Application
code should never depend on a private raw status value.

Error handling does not undo side effects that already happened. For example,
a successful file write followed by a failed flush remains a failed lifecycle
that the caller must recover or report according to the API contract.
