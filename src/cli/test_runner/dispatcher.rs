//! The generated entry of a test executable (ADR-0121).
//!
//! A hosted executable holds every selected test of one test directory. Its
//! `main` reads the index of the test to run from the environment variable
//! [`TEST_INDEX_ENVIRONMENT`] and dispatches on that integer; the runner
//! starts the executable once per test. The index is read through the C
//! library alone (`getenv`, `strlen`, `strspn`, `atoll`, `write`, `exit`),
//! which every hosted target links, so a test executable does not link the
//! Actus runtime for its entry. Freestanding targets have no process
//! environment to select with, so their executables hold one test and `main`
//! calls it directly.

use crate::ast::{MetaAttribute, Program, TopLevelDecl};

use super::collection::DiscoveredTest;

/// Environment variable that names the test an executable runs, as a decimal
/// index.
pub(super) const TEST_INDEX_ENVIRONMENT: &str = "ACTUS_TEST_INDEX";
/// Exit status of an entry that cannot run the requested test.
pub(super) const TEST_ENTRY_ERROR_STATUS: i32 = 125;
/// Start of the stderr message that accompanies [`TEST_ENTRY_ERROR_STATUS`].
pub(super) const TEST_ENTRY_ERROR_PREFIX: &str = "actus test entry error: ";

const FAULT_SYMBOL: &str = "actus_test_publication_fault";

/// Prepares `program` for one executable: drops the test verbs that are not in
/// `tests` and appends the generated entry. Each test comes with the index the
/// runner passes to select it; `dispatch` is false for a single test that
/// `main` calls without selection.
///
/// `main` declares the union of its tests' `meta effect` and `meta capability`
/// attributes. A verb must declare the effects of the verbs it calls, so this
/// is what lets the entry call every test; each test verb is still checked
/// against its own declarations where it is defined.
pub(super) fn append_entry(
    program: &mut Program,
    tests: &[(usize, &DiscoveredTest)],
    dispatch: bool,
) -> Result<(), String> {
    let selected = |name: &str| tests.iter().any(|(_, test)| test.name == name);
    program.declarations.retain(|declaration| match declaration {
        TopLevelDecl::Verb(verb) if verb.metadata.contains(&MetaAttribute::Test) => {
            selected(&verb.name)
        }
        _ => true,
    });
    if program
        .declarations
        .iter()
        .any(|declaration| matches!(declaration, TopLevelDecl::Verb(verb) if verb.name == "main"))
    {
        return Err("meta test program already defines `main`".to_owned());
    }
    let contract = contract_union(program, tests);
    #[cfg(not(feature = "test-fault-injection"))]
    if let Some((_, test)) = tests.iter().find(|(_, test)| test.fault.is_some()) {
        return Err(format!(
            "test `{}` declares a fault; rebuild Actus with `--features test-fault-injection`",
            test.name
        ));
    }
    let entry = parse_entry(tests, dispatch, &contract)?;
    // A test may declare the fault verb itself.
    let declared = |name: &str| {
        program.declarations.iter().any(
            |declaration| matches!(declaration, TopLevelDecl::ExternalVerb(other) if other.name == name),
        )
    };
    let entry = entry
        .declarations
        .into_iter()
        .filter(|declaration| !matches!(declaration, TopLevelDecl::ExternalVerb(external) if declared(&external.name)))
        .collect::<Vec<_>>();
    program.declarations.extend(entry);
    Ok(())
}

/// The union of the `meta effect` and `meta capability` attributes of the
/// selected tests.
fn contract_union(program: &Program, tests: &[(usize, &DiscoveredTest)]) -> Vec<MetaAttribute> {
    let selected = |name: &str| tests.iter().any(|(_, test)| test.name == name);
    let mut contract: Vec<MetaAttribute> = Vec::new();
    for declaration in &program.declarations {
        let TopLevelDecl::Verb(verb) = declaration else { continue };
        if !selected(&verb.name) || !verb.metadata.contains(&MetaAttribute::Test) {
            continue;
        }
        for attribute in &verb.metadata {
            let is_contract =
                matches!(attribute, MetaAttribute::Effect(_) | MetaAttribute::Capability(_));
            if is_contract && !contract.contains(attribute) {
                contract.push(attribute.clone());
            }
        }
    }
    contract
}

/// Parses the entry source and gives `main` the contract of the tests and the
/// entry's C functions their native names.
fn parse_entry(
    tests: &[(usize, &DiscoveredTest)],
    dispatch: bool,
    contract: &[MetaAttribute],
) -> Result<Program, String> {
    let mut entry = crate::parser::parse(crate::lexer::scan(&entry_source(tests, dispatch)).0)
        .map_err(|error| {
            format!("internal error: generated test entry does not parse: {error:?}")
        })?;
    for declaration in &mut entry.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) if verb.name == "main" => verb.metadata = contract.to_vec(),
            TopLevelDecl::ExternalVerb(external) => {
                // The entry's C functions have names of their own, so they
                // cannot collide with declarations of the tests.
                external.native_symbol = external
                    .name
                    .strip_prefix("actus_test_entry_")
                    .filter(|_| external.name != "actus_test_entry_fail")
                    .map(str::to_owned);
            }
            _ => {}
        }
    }
    Ok(entry)
}

/// The Actus source of the entry: `main` and what it needs. Arguments are
/// bound to variables first because the role checks of calls take names.
fn entry_source(tests: &[(usize, &DiscoveredTest)], dispatch: bool) -> String {
    let mut source = String::new();
    if tests.iter().any(|(_, test)| test.fault.is_some()) {
        source
            .push_str(&format!("unsafe extern \"C\" verb {FAULT_SYMBOL}(abs code: Int) -> Int;\n"));
    }
    if dispatch {
        source.push_str(&selection_source());
    }
    source.push_str("verb main() -> Int {\n");
    if dispatch {
        source.push_str("    erg entry_selected: i64 = actus_test_entry_selected();\n");
        for (index, test) in tests {
            source.push_str(&format!(
                "    if entry_selected == {index} {{\n{}    }}\n",
                run_test_source(*index, test)
            ));
        }
        source.push_str("    return actus_test_entry_fail();\n");
    } else {
        source.push_str(&run_test_source(tests[0].0, tests[0].1));
    }
    source.push_str("}\n");
    source
}

/// Runs one test: its fault call, if any, then the test, whose result is the
/// exit status of the process. Bindings are unique within `main`, so the
/// fault code is named by the index of its test.
fn run_test_source(index: usize, test: &DiscoveredTest) -> String {
    let mut source = String::new();
    if let Some(fault) = test.fault {
        source.push_str(&format!(
            "        erg entry_code_{index}: Int = {};\n        {FAULT_SYMBOL}(code: abs entry_code_{index});\n",
            fault.code()
        ));
    }
    source.push_str(&format!("        return {}();\n", test.name));
    source
}

/// Reads the test index from the environment with the C library. A missing or
/// non-decimal value is an entry failure: one message on stderr and exit
/// status 125. A value of more than 18 digits is one too, so `atoll` cannot
/// overflow; any other number is an index, and one that names no test of the
/// executable (above `i32::MAX`, for example) ends in the same failure when
/// `main` finds no branch for it.
fn selection_source() -> String {
    let message = format!(
        "{TEST_ENTRY_ERROR_PREFIX}{TEST_INDEX_ENVIRONMENT} is missing, malformed or does not name a test of this executable"
    );
    format!(
        r#"unsafe extern "C" verb actus_test_entry_getenv(abs name: String) -> Usize;
unsafe extern "C" verb actus_test_entry_strlen(erg text: Usize) -> Usize;
unsafe extern "C" verb actus_test_entry_strspn(erg text: Usize, abs accept: String) -> Usize;
unsafe extern "C" verb actus_test_entry_atoll(erg text: Usize) -> i64;
unsafe extern "C" verb actus_test_entry_write(erg descriptor: Int, abs text: String, erg count: Usize) -> Int;
unsafe extern "C" verb actus_test_entry_exit(erg status: Int) -> Int;
verb actus_test_entry_fail() -> Int {{
    erg entry_message: String = "{message}";
    erg entry_descriptor: Int = 2;
    erg entry_count: Usize = {length};
    erg entry_status: Int = {TEST_ENTRY_ERROR_STATUS};
    actus_test_entry_write(descriptor: erg entry_descriptor, text: abs entry_message, count: erg entry_count);
    actus_test_entry_exit(status: erg entry_status);
    return entry_status;
}}
verb actus_test_entry_selected() -> i64 {{
    erg entry_name: String = "{TEST_INDEX_ENVIRONMENT}";
    erg entry_digits: String = "0123456789";
    erg entry_none: Usize = 0;
    erg entry_longest: Usize = 18;
    erg entry_fallback: i64 = 0;
    erg entry_text: Usize = actus_test_entry_getenv(name: abs entry_name);
    if entry_text == entry_none {{
        actus_test_entry_fail();
        return entry_fallback;
    }}
    erg entry_length: Usize = actus_test_entry_strlen(text: erg entry_text);
    erg entry_decimal: Usize = actus_test_entry_strspn(text: erg entry_text, accept: abs entry_digits);
    if entry_length == entry_none || entry_decimal != entry_length || entry_length > entry_longest {{
        actus_test_entry_fail();
        return entry_fallback;
    }}
    erg entry_index: i64 = actus_test_entry_atoll(text: erg entry_text);
    return entry_index;
}}
"#,
        length = message.len()
    )
}
