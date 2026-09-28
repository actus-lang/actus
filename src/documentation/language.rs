use super::{DocumentationSection, contract::Contract};

pub(super) fn contains_section(
    doc: &str,
    section: DocumentationSection,
    contract: &Contract,
) -> bool {
    let lower = doc.to_ascii_lowercase();
    match section {
        DocumentationSection::Purpose => true,
        DocumentationSection::Ownership => has_all_roles(&lower, contract),
        DocumentationSection::Returns => has_return_language(&lower),
        DocumentationSection::Errors => has_error_language(&lower),
        DocumentationSection::Mutation => has_mutation_language(&lower),
        DocumentationSection::Allocation => has_allocation_language(&lower),
        DocumentationSection::Cleanup => has_cleanup_language(&lower),
        DocumentationSection::Abi => has_abi_language(&lower),
        DocumentationSection::Platform => has_platform_language(&lower),
    }
}

fn has_all_roles(doc: &str, contract: &Contract) -> bool {
    contract.roles.iter().all(|role| role_present(doc, role))
}

fn has_return_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "return",
            "result",
            "option",
            "count",
            "status",
            "value",
            "ok(",
            "err(",
            "success",
            "zero",
            "one",
            "reports",
            "contains",
            "confirms",
            "means",
            "operation",
            "output",
            "written",
            "position",
        ],
    )
}

fn has_error_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "error",
            "failure",
            "failed",
            "invalid",
            "reject",
            "err(",
            "negative",
            "status",
            "typed",
            "result",
            "operation",
            "reports",
        ],
    )
}

fn has_mutation_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "ins",
            "loan",
            "exclusive",
            "caller",
            "mutat",
            "in place",
            "write",
            "fill",
            "append",
            "replace",
            "clear",
            "flush",
            "drop",
            "close",
        ],
    )
}

fn has_allocation_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "alloc",
            "storage",
            "buffer",
            "capacity",
            "reus",
            "zero-allocation",
            "no new",
            "without allocation",
            "without allocating",
            "non-owning",
            "borrow",
            "bytes",
            "caller",
        ],
    )
}

fn has_cleanup_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "drop",
            "cleanup",
            "close",
            "resource",
            "ownership",
            "consume",
            "transfer",
            "owned",
            "handle",
        ],
    )
}

fn has_abi_language(doc: &str) -> bool {
    contains_any(doc, &["abi", "runtime", "bridge", "native", "extern", "status"])
}

fn has_platform_language(doc: &str) -> bool {
    contains_any(
        doc,
        &["platform", "posix", "windows", "host", "operating", "raw", "utf-8", "utf-16", "path"],
    )
}

pub(super) fn has_contradiction(doc: &str, contract: &Contract) -> bool {
    let lower = doc.to_ascii_lowercase();
    contract.roles.iter().any(|role| match *role {
        "abs" => {
            contains_any(&lower, &["takes ownership", "transfer ownership", "owns the buffer"])
        }
        "dat" => contains_any(
            &lower,
            &["does not consume", "never transfers ownership", "borrowed only"],
        ),
        "ins" => contains_any(
            &lower,
            &["retains the pointer", "borrow escapes", "takes permanent ownership"],
        ),
        _ => false,
    })
}

pub(super) fn is_misrepresenting(doc: &str, contract: &Contract) -> bool {
    let lower = doc.to_ascii_lowercase();
    contract.result_return
        && contains_any(&lower, &["always succeeds", "never fails", "cannot fail", "no errors"])
}

fn role_present(doc: &str, role: &str) -> bool {
    match role {
        "erg" => has_erg_language(doc),
        "abs" => has_abs_language(doc),
        "dat" => has_dat_language(doc),
        "ins" => has_ins_language(doc),
        _ => false,
    }
}

fn has_erg_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "erg",
            "owned",
            "owner",
            "by value",
            "scalar",
            "field",
            "flag",
            "handle",
            "bytes",
            "value",
            "status",
            "capacity",
            "length",
            "position",
            "one",
            "regular",
            "read",
            "create",
            "new",
            "pending",
            "current",
            "logical",
            "terminator",
            "separator",
            "storage",
            "integer",
            "byte",
        ],
    )
}

fn has_abs_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "abs",
            "borrow",
            "read-only",
            "non-owning",
            "without consuming",
            "scalar",
            "handle",
            "status",
            "offset",
            "position",
            "origin",
            "mode",
            "byte",
            "path",
            "source",
            "target",
            "input",
            "output",
            "contents",
            "prefix",
            "suffix",
            "self",
            "from",
            "to",
            "extension",
            "name",
            "iter",
        ],
    )
}

fn has_dat_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "dat", "consume", "transfer", "move", "owned", "drop", "cleanup", "output", "options",
            "loaded", "error", "result", "file", "path", "value", "bytes", "discard",
        ],
    )
}

fn has_ins_language(doc: &str) -> bool {
    contains_any(
        doc,
        &[
            "ins",
            "loan",
            "exclusive",
            "in place",
            "mutat",
            "restore",
            "caller storage",
            "caller-owned",
            "self",
            "buffer",
            "target",
            "writer",
            "reader",
            "seek",
            "cursor",
            "operation",
            "iter",
        ],
    )
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
