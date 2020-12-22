use crate::parser::error::ParseError;
use helios_syntax::SyntaxKind;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Event {
    StartNode {
        kind: SyntaxKind,
        forward_parent: Option<usize>,
    },
    AddToken,
    FinishNode,
    Error(ParseError),
    Placeholder,
}
