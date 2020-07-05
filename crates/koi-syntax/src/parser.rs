use crate::lexer::{Lexer, LexerMode};
use crate::node::*;
use crate::source::Span;
use crate::token::*;

pub type Ast = Vec<Node>;

pub struct Parser {
    lexer: Lexer,
    peeked_token: Option<Token>,
}

impl Parser {
    pub fn with(lexer: Lexer) -> Self {
        Self { lexer, peeked_token: None }
    }

    pub fn parse(&mut self) -> Ast {
        let mut nodes = Vec::new();

        while !self.lexer.is_at_end() {
            nodes.push(self.parse_program());
        }

        nodes
    }
}

impl Parser {
    fn peek(&mut self) -> Option<Token> {
        if self.peeked_token.is_none() {
            self.peeked_token = Some(self.lexer.next_token());
        }

        self.peeked_token.clone()
    }

    fn next_token(&mut self) -> Token {
        match self.peeked_token.take() {
            Some(token) => token,
            None => self.lexer.next_token()
        }
    }

    fn check(&mut self, kind: TokenKind) -> bool {
        match self.peek() {
            Some(token) => token.kind == kind,
            None => false
        }
    }

    fn check_all(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.check(kind.clone()) {
                return true;
            }
        }

        false
    }

    fn consume(&mut self, kind: TokenKind) -> Token {
        if let Some(token) = self.peek() {
            if token.kind == kind {
                return self.next_token();
            } else {
                return Token::with(
                    TokenKind::Missing(Box::new(kind)),
                    Span::new(token.span.start, token.span.start)
                );
            }
        }

        panic!("Unhandled case: consuming when peeking has None value");
    }

    fn try_consume(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.next_token();
            true
        } else {
            false
        }
    }
}

impl Parser {
    fn parse_program(&mut self) -> Node {
        self.try_consume(TokenKind::Newline);

        if self.try_consume(TokenKind::Eof) {
            return Node::Eof;
        }

        Node::ExpressionNode(self.parse_expression())
    }

    fn parse_expression(&mut self) -> ExpressionNode {
        if self.try_consume(TokenKind::Eof) {
            return ExpressionNode::Missing(self.lexer.current_pos());
        }

        if self.try_consume(TokenKind::Newline) {
            return ExpressionNode::Unexpected(TokenKind::Newline, self.lexer.current_pos());
        }

        if self.try_consume(TokenKind::Keyword(Keyword::Let)) {
            return self.parse_let_expression();
        }

        if self.try_consume(TokenKind::Keyword(Keyword::If)) {
            return self.parse_if_expression();
        }

        self.parse_binary_expression(0)
    }

    fn parse_let_expression(&mut self) -> ExpressionNode {
        let mut local_binding = LocalBindingNode::new();

        local_binding
            .identifier(self.consume(TokenKind::Identifier))
            .equal_symbol(self.consume(TokenKind::Symbol(Symbol::Eq)))
            .expression(self.parse_expression_block());

        ExpressionNode::LocalBindingNode(local_binding)
    }

    fn parse_if_expression(&mut self) -> ExpressionNode {
        let mut if_expression = IfExpressionNode::new();

        if_expression
            .pattern(self.parse_expression())
            .then_keyword(self.consume(TokenKind::Keyword(Keyword::Then)))
            .expression(self.parse_expression_block());

        self.try_consume(TokenKind::Newline);
        if self.try_consume(TokenKind::Keyword(Keyword::Else)) {
            if_expression.else_clause(self.parse_else_clause());
        }

        ExpressionNode::IfExpressionNode(if_expression)
    }

    fn parse_else_clause(&mut self) -> ExpressionNode {
        if self.try_consume(TokenKind::Keyword(Keyword::If)) {
            self.parse_if_expression()
        } else {
            self.parse_expression_block()
        }
    }

    fn parse_expression_block(&mut self) -> ExpressionNode {
        if self.try_consume(TokenKind::Begin) {
            return self.parse_expression_block_list();
        }

        self.parse_expression()
    }

    fn parse_expression_block_list(&mut self) -> ExpressionNode {
        let mut expressions = Vec::new();
        expressions.push(Box::new(self.parse_expression()));

        while self.check_all(&[
            TokenKind::Newline,
            TokenKind::Symbol(Symbol::Semicolon),
        ]) {
            self.next_token();
            expressions.push(Box::new(self.parse_expression()));
        }

        self.consume(TokenKind::End);

        ExpressionNode::BlockExpressionNode(expressions)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> ExpressionNode {
        if self.check(TokenKind::Begin) {
            return self.parse_expression_block();
        }

        let mut lhs = self.parse_unary_expression();

        loop {
            let operator = match self.peek() {
                Some(token) => match token.kind {
                    TokenKind::Symbol(symbol) => symbol,
                    _ => break,
                },
                None => break
            };

            let (left_precedence, right_precedence) = infix_binding_power(operator);
            if left_precedence < min_precedence {
                break;
            }

            let operator = self.next_token();
            let rhs = self.parse_binary_expression(right_precedence);
            let mut binary_expression = BinaryExpressionNode::new();
            binary_expression.operator(operator).lhs(lhs.clone()).rhs(rhs);
            lhs = ExpressionNode::BinaryExpressionNode(binary_expression);
        }

        lhs
    }

    fn parse_unary_expression(&mut self) -> ExpressionNode {
        if let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Symbol(symbol) => {
                    self.next_token();
                    let right_precedence = prefix_binding_power(symbol);
                    let operand = self.parse_binary_expression(right_precedence);
                    return ExpressionNode::UnaryExpression(token, Box::new(operand));
                },
                _ => return self.parse_primary()
            }
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> ExpressionNode {
        let token = self.next_token();
        match &token.kind {
            TokenKind::Identifier => {
                ExpressionNode::Identifier
            },
            TokenKind::Keyword(Keyword::False) => {
                ExpressionNode::LiteralNode(LiteralNode::Boolean(false))
            },
            TokenKind::Keyword(Keyword::True) => {
                ExpressionNode::LiteralNode(LiteralNode::Boolean(true))
            },
            TokenKind::Literal(literal) => match literal {
                Literal::Integer(_) => {
                    ExpressionNode::LiteralNode(LiteralNode::Integer(token))
                },
                Literal::Float(_) => {
                    ExpressionNode::LiteralNode(LiteralNode::Float(token))
                },
                l => unimplemented!("Literal {:?}", l)
            },
            TokenKind::GroupingStart(delimiter) => {
                self.lexer.push_mode(LexerMode::Grouping);
                let mut grouped_expression = GroupedExpressionNode::new();

                grouped_expression
                    .start_delimiter(token.clone())
                    .expression(self.parse_expression())
                    .end_delimiter(self.consume(TokenKind::GroupingEnd(delimiter.clone())));

                ExpressionNode::GroupedExpressionNode(grouped_expression)
            },
            k => ExpressionNode::Unexpected(k.clone(), self.lexer.current_pos())
        }
    }
}

fn prefix_binding_power(symbol: Symbol) -> u8 {
    match symbol {
        Symbol::Minus | Symbol::Bang => 9,
        _ => panic!("Bad operator: {:?}", symbol)
    }
}

fn infix_binding_power(symbol: Symbol) -> (u8, u8) {
    match symbol {
        Symbol::Eq | Symbol::BangEq => (2, 1),
        Symbol::Lt | Symbol::Gt | Symbol::LtEq | Symbol::GtEq => (3, 4),
        Symbol::Plus | Symbol::Minus => (5, 6),
        Symbol::Asterisk | Symbol::ForwardSlash => (7, 8),
        _ => panic!("Bad operator: {:?}", symbol)
    }
}
