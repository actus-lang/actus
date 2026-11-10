use crate::ast::MetaAttribute;
use crate::lexer::SourceSpan;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(in crate::parser) fn reject_duplicate_basic_metadata(
        &self,
        metadata: &[MetaAttribute],
    ) -> Result<(), ParseError> {
        let duplicate =
            if metadata.iter().filter(|attribute| matches!(attribute, MetaAttribute::Test)).count()
                > 1
            {
                Some("test")
            } else if metadata
                .iter()
                .filter(|attribute| matches!(attribute, MetaAttribute::MustUse(_)))
                .count()
                > 1
            {
                Some("must_use")
            } else if metadata
                .iter()
                .filter(|attribute| matches!(attribute, MetaAttribute::Volatile))
                .nth(1)
                .is_some()
            {
                Some("volatile")
            } else if metadata
                .iter()
                .filter(|attribute| matches!(attribute, MetaAttribute::ZeroOnDrop))
                .nth(1)
                .is_some()
            {
                Some("zero_on_drop")
            } else if metadata
                .iter()
                .filter(|attribute| matches!(attribute, MetaAttribute::StackLimit(_)))
                .nth(1)
                .is_some()
            {
                Some("stack_limit")
            } else {
                duplicate_repr_metadata(metadata)
            };
        let Some(name) = duplicate else { return Ok(()) };
        Err(ParseError {
            code: ParseErrorCode::DuplicateMetadata,
            kind: ParseErrorKind::DuplicateMetadata { name: name.to_owned() },
            span: self.peek().map(|token| token.span).unwrap_or(SourceSpan::new(0, 0)),
        })
    }
}

fn duplicate_repr_metadata(metadata: &[MetaAttribute]) -> Option<&'static str> {
    let mut repr = crate::ast::Repr::new(false, false);
    for attribute in metadata {
        let MetaAttribute::Repr(current) = attribute else { continue };
        if (current.is_c() && repr.is_c())
            || (current.is_packed() && repr.is_packed())
            || (current.is_transparent() && repr.is_transparent())
            || (current.alignment().is_some() && repr.alignment().is_some())
        {
            return Some("repr");
        }
        repr = repr.combine(*current);
    }
    None
}
