use crate::lexer::{Lexer, LexToken};
use crate::common::{BinOp,UnaryOp};

use std::iter::Peekable;

#[derive(Debug)]
pub struct Module<'a> {
    pub name: &'a str,
    pub arg_names: Vec<&'a str>,
    pub stmts: Vec<Statement<'a>>
}

#[derive(Debug)]
pub enum Statement<'a> {
    Terminator,
    Empty,
    VarBinding(Vec<&'a str>,Expr<'a>),
    Output(Vec<Expr<'a>>)
}

#[derive(Debug)]
pub enum Expr<'a> {
    Ident(&'a str),
    Constant(i64),
    BinOp(Box<Expr<'a>>,BinOp,Box<Expr<'a>>),
    UnOp(UnaryOp,Box<Expr<'a>>),
    If(Box<Expr<'a>>,Box<Expr<'a>>,Option<Box<Expr<'a>>>),
    Match(Box<Expr<'a>>,Vec<(Expr<'a>,Expr<'a>)>)
}

struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>
}

impl<'a> Parser<'a> {
    fn new(lexer: Lexer<'a>) -> Self {
        Self{lexer: lexer.peekable()}
    }

    fn take(&mut self, tok: LexToken) {
        let present = self.next();
        if present != tok {
            panic!("Expected {:?}, found {:?}.",tok,present);
        }
    }

    fn take_ident(&mut self) -> &'a str {
        let present = self.next();
        if let LexToken::Ident(ident_str) = present {
            ident_str
        } else {
            panic!("Expected ident, found {:?}.",present);
        }
    }

    fn take_comma_or_close_paren(&mut self) -> bool {
        let present = self.next();
        match present {
            LexToken::OpComma => false,
            LexToken::OpParenClose => true,
            _ => panic!("Expected comma or close paren, found {:?}.",present)
        }
    }

    fn next(&mut self) -> LexToken<'a> {
        self.lexer.next().expect("Expected token, found EOF.")
    }

    fn peek(&mut self) -> LexToken<'a> {
        *self.lexer.peek().expect("Expected token, found EOF.")
    }

    fn is_eof(&mut self) -> bool {
        self.lexer.peek().is_none()   
    }
}

pub fn parse<'a>(source: &'a str) -> Vec<Module<'a>> {

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    // Module declaration
    let mut modules = Vec::new();
    while !parser.is_eof() {
        parser.take(LexToken::KeyMod);
        let mod_name = parser.take_ident();
        let mut mod_args = Vec::new();
        let mut mod_stmts = Vec::new();
        
        // Arguments
        parser.take(LexToken::OpParenOpen);
        if parser.peek() != LexToken::OpParenClose {
            loop {
                mod_args.push(parser.take_ident());
                if parser.take_comma_or_close_paren() {
                    break;
                }
            }
        } else {
            parser.take(LexToken::OpParenClose);
        }
        
        parser.take(LexToken::OpBraceOpen);
        loop {
            let stmt = parse_stmt(&mut parser);
            match stmt {
                Statement::Empty => (),
                Statement::Terminator => break,
                _ => mod_stmts.push(stmt)
            }
        }
        modules.push(Module{
            name: mod_name,
            arg_names: mod_args,
            stmts: mod_stmts
        });
    }

    modules
}

fn parse_stmt<'a>(parser: &mut Parser<'a>) -> Statement<'a> {
    let tok = parser.next();
    match tok {
        LexToken::KeyOutput => {
            let mut out_args = Vec::new();
            parser.take(LexToken::OpParenOpen);
            // Don't worry about the empty case, why output nothing?
            loop {
                out_args.push(parse_expr(parser));
                if parser.take_comma_or_close_paren() {
                    break;
                }
            }
            Statement::Output(out_args)
        },
        LexToken::KeyLet => {
            let ident = parser.take_ident();
            parser.take(LexToken::OpAssign);
            Statement::VarBinding(vec!(ident),parse_expr(parser))
        },
        LexToken::OpSemicolon => Statement::Empty,
        LexToken::OpBraceClose => Statement::Terminator,
        _ => panic!("Expected statment, found {:?}.",tok)
    }
}

fn parse_expr<'a>(parser: &mut Parser<'a>) -> Expr<'a> {

    let mut expr_stack: Vec<Expr> = Vec::new();
    let mut op_stack: Vec<BinOp> = Vec::new();

    expr_stack.push(parse_leaf(parser));

    loop {
        // try parsing an operator, or end the expression
        let next_tok = parser.peek();

        let new_op = match next_tok {
            LexToken::OpAdd => BinOp::Add,
            LexToken::OpSub => BinOp::Sub,
            LexToken::OpMul => BinOp::Mul,
            LexToken::OpDiv => BinOp::Div,
            LexToken::OpMod => BinOp::Mod,
            LexToken::OpPower => BinOp::Power,

            LexToken::OpBitAnd => BinOp::BitAnd,
            LexToken::OpBitOr => BinOp::BitOr,
            LexToken::OpBitXor => BinOp::BitXor,
            LexToken::OpShiftLeft => BinOp::ShiftLeft,
            LexToken::OpShiftRight => BinOp::ShiftRight,

            LexToken::OpCmpEq => BinOp::CmpEq,
            LexToken::OpCmpNeq => BinOp::CmpNeq,
            LexToken::OpCmpLt => BinOp::CmpLt,
            LexToken::OpCmpGt => BinOp::CmpGt,
            LexToken::OpCmpLeq => BinOp::CmpLeq,
            LexToken::OpCmpGeq => BinOp::CmpGeq,

            // sane expression terminators
            LexToken::OpParenClose |
            LexToken::OpSemicolon |
            LexToken::OpComma |
            LexToken::OpMatchArrow => break,
            _ => panic!("Expected operator, found {:?}",next_tok)
        };

        while let Some(top_op) = op_stack.last() {
            if top_op.prec() <= new_op.prec() {
                let op = op_stack.pop().unwrap();
                let rhs = expr_stack.pop().unwrap();
                let lhs = expr_stack.pop().unwrap();

                let bin_expr = Expr::BinOp(Box::new(lhs),op,Box::new(rhs));
                expr_stack.push(bin_expr);
            } else {
                break;
            }
        }
        op_stack.push(new_op);

        // advance
        parser.next();

        // rhs of the parsed operator
        expr_stack.push(parse_leaf(parser));
    }

    while let Some(op) = op_stack.pop() {
        let rhs = expr_stack.pop().unwrap();
        let lhs = expr_stack.pop().unwrap();
        let bin_expr = Expr::BinOp(Box::new(lhs),op,Box::new(rhs));
        expr_stack.push(bin_expr);
    }

    assert_eq!(expr_stack.len(),1);
    expr_stack.pop().unwrap()
}

fn parse_leaf<'a>(parser: &mut Parser<'a>) -> Expr<'a> {
    let tok = parser.next();

    match tok {
        // TODO could be a module call!
        LexToken::Ident(id) => Expr::Ident(id),
        LexToken::KeyIf => {
            parser.take(LexToken::OpParenOpen);
            let cond = parse_expr(parser);
            parser.take(LexToken::OpComma);
            let val_true = parse_expr(parser);
            
            let val_false = if parser.peek() == LexToken::OpComma {
                parser.take(LexToken::OpComma);
                let val = parse_expr(parser);
                parser.take(LexToken::OpParenClose);
                Some(Box::new(val))
            } else {
                parser.take(LexToken::OpParenClose);
                None
            };

            Expr::If(Box::new(cond),Box::new(val_true),val_false)
        },
        LexToken::KeyMatch => {
            parser.take(LexToken::OpParenOpen);
            let in_expr = parse_expr(parser);
            parser.take(LexToken::OpParenClose);

            parser.take(LexToken::OpBraceOpen);
            let mut match_list = Vec::new();
            loop {
                if parser.peek() == LexToken::OpBraceClose {
                    parser.next();
                    break;
                }
                let test_expr = parse_expr(parser);
                parser.take(LexToken::OpMatchArrow);
                let res_expr = parse_expr(parser);

                match_list.push((test_expr,res_expr));
                
                let next_token = parser.next();
                if next_token == LexToken::OpBraceClose {
                    break;
                } else if next_token != LexToken::OpComma {
                    panic!("Expected ',' or '}}', found {:?}.",next_token);
                }
            }

            Expr::Match(Box::new(in_expr),match_list)
        },
        LexToken::Number(num) => Expr::Constant(num),
        LexToken::OpParenOpen => {
            // This *can* be done in the normal expression parser without recursion, but it's cleaner to do here.
            let expr = parse_expr(parser);
            parser.take(LexToken::OpParenClose);
            expr
        },
        LexToken::OpSub => {
            Expr::UnOp(UnaryOp::Negate, Box::new(parse_expr(parser)))
        },
        _ => panic!("Expected expression, found {:?}.",tok)
    }
}
