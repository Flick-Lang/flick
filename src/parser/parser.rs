use crate::lexer::token::AssignmentSymbol::*;
use crate::lexer::token::ComparatorSymbol::*;
use crate::lexer::token::OperatorSymbol::*;
use crate::lexer::token::{Token, Type};
use crate::parser::ast::*;

// TODO(tbreydo): get rid of Parser object (just use functions)
pub struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn next_token(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.cursor);
        self.cursor += 1;
        token
    }

    /// Returns a reference to the next() value without advancing the cursor.
    fn peek_token(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.cursor + (n - 1)) // n-1 to fix indexing
    }

    fn skip_token(&mut self) {
        self.cursor += 1;
    }

    pub fn parse_program(&mut self) -> Program {
        let mut func_defs = Vec::new();
        while let Some(func_def) = self.parse_func_def() {
            func_defs.push(func_def);
        }
        Program { func_defs }
    }

    fn skip_newlines_comments_and_docstrings(&mut self) {
        // todo take into account the fact that docstring CAN appear in parse tree
        // enjoy this beautiful formatting <3
        while let Some(Token::Newline | Token::Comment(_) | Token::Docstring(_)) =
            self.peek_token(1)
        {
            self.skip_token();
        }
    }

    fn parse_func_def(&mut self) -> Option<FuncDef> {
        self.skip_newlines_comments_and_docstrings();

        match self.peek_token(1)? {
            Token::Fn => self.skip_token(),
            token => panic!("Expected Token::Fn but received {:?}", token),
        }

        let name = self.parse_identifier();
        let params = self.parse_func_params();

        let return_type = match self.peek_token(1) {
            Some(Token::LSquirly) => Type::Void,
            Some(Token::Type(_)) => self.parse_type(),
            Some(t) => panic!(
                "Expected return type for function '{}' but received {:?}",
                name, t
            ),
            None => panic!(
                "Expected return type for function '{}' but file ended",
                name
            ),
        };

        let body = self.parse_body();

        Some(FuncDef {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_func_params(&mut self) -> Vec<FuncParam> {
        self.assert_next_token(Token::LParen);

        let mut params = Vec::new();

        if let Some(Token::RParen) = self.peek_token(1) {
            self.skip_token();
            return params;
        }

        loop {
            let param_type = self.parse_type();
            let param_name = self.parse_identifier();

            let func_param = FuncParam {
                param_type,
                param_name,
            };

            params.push(func_param);

            match self.next_token() {
                Some(Token::RParen) => break,
                Some(Token::Comma) => continue,
                Some(token) => panic!("Expected ')' but received {:?}", token),
                None => panic!("Expected ')' but file ended"),
            }
        }

        params
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        self.skip_newlines_comments_and_docstrings();

        let statement = match self.peek_token(1)? {
            Token::Type(_) => Statement::VarDeclarations(self.parse_var_decs()),
            Token::While => Statement::WhileLoop(self.parse_while_loop()),
            Token::Fn => panic!("Nested function definitions are not allowed"),
            Token::Ret => Statement::Return(self.parse_return_statement()),
            _ => Statement::Expr(self.parse_expr()),
        };

        match self.next_token() {
            Some(Token::Newline | Token::Semicolon) | None => Some(statement),
            Some(token) => panic!("Expected newline or EOF but received {:?}", token),
        }
    }

    // TODO: Maybe make this a macro?
    fn assert_next_token(&mut self, expected: Token) {
        match self.next_token() {
            Some(token) if *token == expected => (),
            Some(token) => panic!("Expected {:?} but received {:?}", expected, token),
            None => panic!("Expected {:?} but file ended", expected),
        }
    }

    fn parse_type(&mut self) -> Type {
        match self.next_token() {
            Some(Token::Type(var_type)) => *var_type,
            Some(t) => panic!("Expected type of variable but received {:?}", t),
            None => panic!("Expected type of variable but file ended"),
        }
    }

    fn parse_identifier(&mut self) -> String {
        match self.next_token() {
            Some(Token::Identifier(id)) => id.clone(), // TODO: Can we somehow get rid of this clone
            Some(t) => panic!("Expected identifier but received {:?}", t),
            None => panic!("Expected identifier but received end of file"),
        }
    }

    fn parse_var_decs(&mut self) -> Vec<VarDeclaration> {
        let var_type = self.parse_type();

        let mut var_decs = Vec::new();

        loop {
            let var_name = self.parse_identifier();

            let var_value = match self.peek_token(1) {
                Some(Token::AssignmentSymbol(Eq)) => {
                    self.skip_token();
                    Some(self.parse_expr())
                }
                _ => None,
            };

            let var_dec = VarDeclaration {
                var_name,
                var_type,
                var_value,
            };

            var_decs.push(var_dec);

            match self.peek_token(1) {
                Some(Token::Comma) => self.skip_token(),
                _ => break,
            }
        }

        var_decs
    }

    fn parse_body(&mut self) -> Vec<Statement> {
        let mut body = Vec::new();
        self.assert_next_token(Token::LSquirly);

        while let Some(token) = self.peek_token(1) {
            if *token == Token::RSquirly {
                break;
            }

            match self.parse_statement() {
                Some(statement) => body.push(statement),
                None => panic!("Expected body to be closed ('}}') but file ended"),
            }
        }
        self.assert_next_token(Token::RSquirly);
        body
    }

    fn parse_while_loop(&mut self) -> WhileLoop {
        self.assert_next_token(Token::While);

        let condition = self.parse_expr();
        let body = self.parse_body();

        WhileLoop { condition, body }
    }

    fn parse_return_statement(&mut self) -> Option<Expr> {
        self.assert_next_token(Token::Ret);

        match self.peek_token(1)? {
            Token::Newline | Token::Semicolon => None,
            _ => Some(self.parse_expr()),
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_assignment_expr()
    }

    fn parse_assignment_expr(&mut self) -> Expr {
        match (self.peek_token(1), self.peek_token(2)) {
            (Some(Token::Identifier(_)), Some(Token::AssignmentSymbol(_))) => {}
            _ => return self.parse_logical_or_expr(),
        }

        let name = self.parse_identifier();
        let operator_symbol = self.next_token().unwrap();

        let name_expr = Expr::Identifier(name.clone());

        let value = match operator_symbol {
            Token::AssignmentSymbol(PlusEq) => Expr::Binary(Binary {
                left: Box::new(name_expr),
                operator: BinaryOperator::Add,
                right: Box::new(self.parse_expr()),
            }),
            Token::AssignmentSymbol(TimesEq) => Expr::Binary(Binary {
                left: Box::new(name_expr),
                operator: BinaryOperator::Multiply,
                right: Box::new(self.parse_expr()),
            }),
            Token::AssignmentSymbol(MinusEq) => Expr::Binary(Binary {
                left: Box::new(name_expr),
                operator: BinaryOperator::Subtract,
                right: Box::new(self.parse_expr()),
            }),
            Token::AssignmentSymbol(DivideEq) => Expr::Binary(Binary {
                left: Box::new(name_expr),
                operator: BinaryOperator::Divide,
                right: Box::new(self.parse_expr()),
            }),
            Token::AssignmentSymbol(Eq) => self.parse_expr(),
            _ => unreachable!(),
        };

        Expr::Assign(Assign {
            name,
            value: Box::new(value),
        })
    }

    fn parse_logical_or_expr(&mut self) -> Expr {
        self.parse_logical_and_expr()
    }

    fn parse_logical_and_expr(&mut self) -> Expr {
        self.parse_comparison_expression()
    }

    fn parse_comparison_expression(&mut self) -> Expr {
        let left = self.parse_add_sub_expr();

        let operator = match self.peek_token(1) {
            Some(Token::ComparatorSymbol(s)) => BinaryOperator::from(*s),
            _ => return left,
        };

        self.skip_token(); // skip the compare symbol

        let right = self.parse_add_sub_expr();

        if let Some(Token::ComparatorSymbol(_)) = self.peek_token(1) {
            // TODO: print a useful error message for the user
            panic!("Comparison operators cannot be chained")
        }

        Expr::Binary(Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }

    fn parse_add_sub_expr(&mut self) -> Expr {
        let mut left_expr_so_far = self.parse_mul_div_expr();

        while let Some(Token::OperatorSymbol(s @ (Plus | Minus))) = self.peek_token(1) {
            let operator = BinaryOperator::from(*s);
            self.skip_token();
            let right = self.parse_mul_div_expr();

            left_expr_so_far = Expr::Binary(Binary {
                left: Box::new(left_expr_so_far),
                operator,
                right: Box::new(right),
            })
        }

        left_expr_so_far
    }

    fn parse_mul_div_expr(&mut self) -> Expr {
        let mut left_expr_so_far = self.parse_primary_expr();

        while let Some(Token::OperatorSymbol(s @ (Asterisk | Slash))) = self.peek_token(1) {
            let operator = BinaryOperator::from(*s);
            self.skip_token();
            let right = self.parse_primary_expr();

            left_expr_so_far = Expr::Binary(Binary {
                left: Box::new(left_expr_so_far),
                operator,
                right: Box::new(right),
            })
        }

        left_expr_so_far
    }

    fn parse_primary_expr(&mut self) -> Expr {
        match (self.peek_token(1), self.peek_token(2)) {
            (Some(Token::LParen), _) => {
                self.skip_token();
                let expr = self.parse_expr();
                self.assert_next_token(Token::RParen);
                expr
            }
            (Some(Token::Identifier(_)), Some(Token::LParen)) => self.parse_call_expr(),
            _ => self.parse_atom(),
        }
    }

    fn parse_call_expr(&mut self) -> Expr {
        let identifier = self.parse_identifier();
        match self.peek_token(1) {
            Some(Token::LParen) => Expr::Call(Call {
                function_name: identifier,
                args: self.parse_args(),
            }),
            _ => Expr::Identifier(identifier),
        }
    }

    fn parse_args(&mut self) -> Vec<Expr> {
        self.assert_next_token(Token::LParen);

        let mut args = Vec::new();

        if let Some(Token::RParen) = self.peek_token(1) {
            self.skip_token();
            return args;
        }

        loop {
            args.push(self.parse_expr());

            match self.next_token() {
                Some(Token::RParen) => break,
                Some(Token::Comma) => continue,
                Some(token) => panic!("Expected ')' but received {:?}", token),
                None => panic!("Expected ')' but file ended"),
            }
        }

        args
    }

    fn parse_atom(&mut self) -> Expr {
        match self.next_token() {
            Some(Token::Identifier(id)) => Expr::Identifier(id.clone()),
            Some(Token::I64Literal(n)) => Expr::I64Literal(*n),
            // todo Some(Token::StrLiteral())
            Some(token) => panic!("Expected identifier or literal but received {:?}", token),
            None => panic!("Expected identifier or literal but file ended"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_declaration() {
        let tokens = vec![
            Token::Type(Type::I64),
            Token::Identifier("x".to_string()),
            Token::AssignmentSymbol(Eq),
            Token::I64Literal(5),
            Token::Comma,
            Token::Identifier("a".to_string()),
            Token::Comma,
            Token::Identifier("m".to_string()),
            Token::AssignmentSymbol(Eq),
            Token::I64Literal(3),
        ];
        let expected = Some(Statement::VarDeclarations(vec![
            VarDeclaration {
                var_name: "x".to_string(),
                var_type: Type::I64,
                var_value: Some(Expr::I64Literal(5)),
            },
            VarDeclaration {
                var_name: "a".to_string(),
                var_type: Type::I64,
                var_value: None,
            },
            VarDeclaration {
                var_name: "m".to_string(),
                var_type: Type::I64,
                var_value: Some(Expr::I64Literal(3)),
            },
        ]));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn var_modification() {
        let tokens = vec![
            Token::Identifier("num".to_string()),
            Token::AssignmentSymbol(Eq),
            Token::Identifier("a".to_string()),
            Token::AssignmentSymbol(Eq),
            Token::I64Literal(10),
        ];
        let expected = Some(Statement::Expr(Expr::Assign(Assign {
            name: "num".to_string(),
            value: Box::new(Expr::Assign(Assign {
                name: "a".to_string(),
                value: Box::new(Expr::I64Literal(10)),
            })),
        })));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn empty_while_loop() {
        let tokens = vec![
            Token::While,
            Token::Identifier("i".to_string()),
            Token::ComparatorSymbol(LessThanOrEqualTo),
            Token::Identifier("N".to_string()),
            Token::LSquirly,
            Token::RSquirly,
        ];
        let expected = Some(Statement::WhileLoop(WhileLoop {
            condition: Expr::Binary(Binary {
                left: Box::new(Expr::Identifier("i".to_string())),
                operator: BinaryOperator::LessOrEqualTo,
                right: Box::new(Expr::Identifier("N".to_string())),
            }),
            body: vec![],
        }));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn order_of_operations() {
        let tokens = vec![
            Token::I64Literal(10),
            Token::OperatorSymbol(Plus),
            Token::I64Literal(3),
            Token::OperatorSymbol(Asterisk),
            Token::I64Literal(8),
            Token::OperatorSymbol(Slash),
            Token::I64Literal(4),
            Token::OperatorSymbol(Minus),
            Token::I64Literal(13),
            Token::OperatorSymbol(Plus),
            Token::I64Literal(5),
        ];
        let expected = Some(Statement::Expr(Expr::Binary(Binary {
            left: Box::new(Expr::Binary(Binary {
                left: Box::new(Expr::Binary(Binary {
                    left: Box::new(Expr::I64Literal(10)),
                    operator: BinaryOperator::Add,
                    right: Box::new(Expr::Binary(Binary {
                        left: Box::new(Expr::Binary(Binary {
                            left: Box::new(Expr::I64Literal(3)),
                            operator: BinaryOperator::Multiply,
                            right: Box::new(Expr::I64Literal(8)),
                        })),
                        operator: BinaryOperator::Divide,
                        right: Box::new(Expr::I64Literal(4)),
                    })),
                })),
                operator: BinaryOperator::Subtract,
                right: Box::new(Expr::I64Literal(13)),
            })),
            operator: BinaryOperator::Add,
            right: Box::new(Expr::I64Literal(5)),
        })));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn parenthetical_expression() {
        let tokens = vec![
            Token::I64Literal(9),
            Token::OperatorSymbol(Asterisk),
            Token::LParen,
            Token::I64Literal(2),
            Token::OperatorSymbol(Plus),
            Token::I64Literal(3),
            Token::RParen,
        ];
        let expected = Some(Statement::Expr(Expr::Binary(Binary {
            left: Box::new(Expr::I64Literal(9)),
            operator: BinaryOperator::Multiply,
            right: Box::new(Expr::Binary(Binary {
                left: Box::new(Expr::I64Literal(2)),
                operator: BinaryOperator::Add,
                right: Box::new(Expr::I64Literal(3)),
            })),
        })));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn spacing() {
        let tokens = vec![
            Token::Newline,
            Token::Newline,
            Token::Newline,
            Token::Identifier("a".to_string()),
            Token::Newline,
            Token::Newline,
        ];
        let expected = Some(Statement::Expr(Expr::Identifier("a".to_string())));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn function_call() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LParen,
            Token::Identifier("f".to_string()),
            Token::LParen,
            Token::I64Literal(1),
            Token::RParen,
            Token::Comma,
            Token::I64Literal(10),
            Token::Comma,
            Token::I64Literal(20),
            Token::RParen,
        ];
        let expected = Some(Statement::Expr(Expr::Call(Call {
            function_name: "print".to_string(),
            args: vec![
                Expr::Call(Call {
                    function_name: "f".to_string(),
                    args: vec![Expr::I64Literal(1)],
                }),
                Expr::I64Literal(10),
                Expr::I64Literal(20),
            ],
        })));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn function_definition() {
        let tokens = vec![
            Token::Fn,
            Token::Identifier("test".to_string()),
            Token::LParen,
            Token::Type(Type::I64),
            Token::Identifier("a".to_string()),
            Token::RParen,
            Token::Type(Type::I64),
            Token::LSquirly,
            Token::RSquirly,
        ];
        let expected = Program {
            func_defs: vec![FuncDef {
                name: "test".to_string(),
                params: vec![FuncParam {
                    param_type: Type::I64,
                    param_name: "a".to_string(),
                }],
                return_type: Type::I64,
                body: vec![],
            }],
        };

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_program();

        assert_eq!(expected, ast);
    }

    #[test]
    fn return_statement() {
        let tokens = vec![
            Token::Ret,
            Token::Identifier("x".to_string()),
            Token::OperatorSymbol(Plus),
            Token::I64Literal(5),
        ];
        let expected = Some(Statement::Return(Some(Expr::Binary(Binary {
            left: Box::new(Expr::Identifier("x".to_string())),
            operator: BinaryOperator::Add,
            right: Box::new(Expr::I64Literal(5)),
        }))));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }

    #[test]
    fn plus_eq() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::AssignmentSymbol(PlusEq),
            Token::I64Literal(5),
        ];
        let expected = Some(Statement::Expr(Expr::Assign(Assign {
            name: "x".to_string(),
            value: Box::new(Expr::Binary(Binary {
                left: Box::new(Expr::Identifier("x".to_string())),
                operator: BinaryOperator::Add,
                right: Box::new(Expr::I64Literal(5)),
            })),
        })));

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_statement();

        assert_eq!(expected, ast);
    }
}
