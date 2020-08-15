use {
    crate::parser::Rule,
    pest::{error::Error as PestError, Span},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{0}` is not a valid action")]
    InvalidAction(String),
    #[error("Unexpected end of stream")]
    UnexpectedEOS,
    #[error("Unexpected pair in stream: {0:?}")]
    UnexpectedPair(ErrorSpan),

    #[error("Parse error")]
    Parse(#[from] PestError<Rule>),
}

#[derive(Debug)]
pub struct ErrorSpan {
    value: String,
    start: usize,
    end: usize,
}

impl From<Span<'_>> for ErrorSpan {
    fn from(span: Span<'_>) -> Self {
        ErrorSpan {
            value: span.as_str().to_string(),
            start: span.start(),
            end: span.end(),
        }
    }
}
