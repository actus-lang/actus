//! Prefix operators bind tighter than `as`, which binds tighter than every binary operator;
//! postfix forms (calls, fields, indexing, `?`) bind tightest.
use actus::ast::{Expr, Stmt, TopLevelDecl};
use actus::lexer::scan;
use actus::parser::parse;

fn shape(expression: &Expr) -> String {
    match expression {
        Expr::Integer { value, .. } => value.clone(),
        Expr::Identifier { name, .. } => name.clone(),
        Expr::Grouping { expression, .. } => format!("({})", shape(expression)),
        Expr::Unary { operator, expression, .. } => {
            format!("{operator:?}[{}]", shape(expression))
        }
        Expr::Cast { expression, target, .. } => {
            format!("Cast[{} as {}]", shape(expression), target.name)
        }
        Expr::Binary { left, operator, right, .. } => {
            format!("{operator:?}[{}, {}]", shape(left), shape(right))
        }
        Expr::MethodCall { receiver, method, .. } => format!("{}.{method}()", shape(receiver)),
        Expr::Index { target, index, .. } => format!("{}[{}]", shape(target), shape(index)),
        Expr::Try { expression, .. } => format!("{}?", shape(expression)),
        other => panic!("unexpected expression in shape: {other:?}"),
    }
}

fn returned_shape(expression: &str) -> String {
    let source = format!("verb main() {{ return {expression}; }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).unwrap_or_else(|error| panic!("{expression}: {error:?}"));
    let TopLevelDecl::Verb(verb) = &program.declarations[0] else { panic!("expected verb") };
    let Stmt::Return { value: Some(value), .. } = &verb.body.statements[0] else {
        panic!("expected return with a value");
    };
    shape(value)
}

#[test]
fn prefix_operators_bind_tighter_than_as() {
    let cases = [
        ("-1 as u8", "Cast[Negate[1] as u8]"),
        ("-128 as i8", "Cast[Negate[128] as i8]"),
        ("-128i32 as i8", "Cast[Negate[128] as i8]"),
        ("-x as i64", "Cast[Negate[x] as i64]"),
        ("-%x as u8", "Cast[WrappingNegate[x] as u8]"),
        ("~b as u16", "Cast[BitwiseNot[b] as u16]"),
        ("!f as u8", "Cast[LogicalNot[f] as u8]"),
        ("- -1 as i8", "Cast[Negate[Negate[1]] as i8]"),
        ("-(1 as u8)", "Negate[(Cast[1 as u8])]"),
        ("(-1) as u8", "Cast[(Negate[1]) as u8]"),
    ];
    for (source, expected) in cases {
        assert_eq!(returned_shape(source), expected, "{source}");
    }
}

#[test]
fn as_stays_tighter_than_binary_operators_and_looser_than_postfix() {
    let cases = [
        ("a - 1 as u8", "Subtract[a, Cast[1 as u8]]"),
        ("x as u8 + 1", "Add[Cast[x as u8], 1]"),
        ("-a as u8 + -b as u8", "Add[Cast[Negate[a] as u8], Cast[Negate[b] as u8]]"),
        ("-v.len() as u8", "Cast[Negate[v.len()] as u8]"),
        ("-v[0] as u8", "Cast[Negate[v[0]] as u8]"),
        ("x as u16 as u8", "Cast[Cast[x as u16] as u8]"),
        ("-x as i64 as i32", "Cast[Cast[Negate[x] as i64] as i32]"),
    ];
    for (source, expected) in cases {
        assert_eq!(returned_shape(source), expected, "{source}");
    }
}
