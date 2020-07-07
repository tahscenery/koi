use crate::token::Token;
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnexpectedExpressionNode {
    pub(crate) token: Token,
}

impl ExpressionNode for UnexpectedExpressionNode {
    fn span(&self) -> Span {
        self.token.span
    }
}
