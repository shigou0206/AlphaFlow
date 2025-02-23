//! JMESPath 解析器，使用 Pratt 解析器
//!
//! 本模块将一个 JMESPath 表达式字符串解析成抽象语法树（AST）。

use std::collections::VecDeque;
use crate::ast::{Ast, Comparator, KeyValuePair};
use crate::lexer::{tokenize, Token, TokenTuple};
use crate::{ErrorReason, JmespathError};

// 引入日志宏
use log::{trace, debug};

/// 解析结果类型
pub type ParseResult = Result<Ast, JmespathError>;

/// 将一个 JMESPath 表达式解析为 AST。
pub fn parse(expr: &str) -> ParseResult {
    trace!("parse: start expr={:?}", expr);
    let tokens = tokenize(expr)?;
    trace!("parse: tokens => {:?}", tokens);
    let mut parser = Parser::new(tokens, expr);
    let result = parser.parse();
    trace!("parse: final result => {:?}", result);
    result
}

/// 当 token 的左结合力低于此值时，停止投影解析
const PROJECTION_STOP: usize = 10;

struct Parser<'a> {
    /// 解析得到的 token 队列
    token_queue: VecDeque<TokenTuple>,
    /// 共享的 EOF token
    eof_token: Token,
    /// 当前解析的表达式
    expr: &'a str,
    /// 当前解析的位置（字符偏移）
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: VecDeque<TokenTuple>, expr: &'a str) -> Parser<'a> {
        Parser {
            token_queue: tokens,
            eof_token: Token::Eof,
            offset: 0,
            expr,
        }
    }

    #[inline]
    fn parse(&mut self) -> ParseResult {
        trace!("Parser::parse: begin expr={:?}", self.expr);
        let result = self.expr(0)?;
        // 解析完毕后，检查队列中是否还有 token
        match self.peek(0) {
            Token::Eof => {
                trace!("Parser::parse: done => AST={:?}", result);
                Ok(result)
            }
            t => {
                trace!("Parser::parse: leftover token={:?}", t);
                Err(self.err(t, "Did not parse the complete expression", true))
            }
        }
    }

    #[inline]
    fn advance(&mut self) -> Token {
        let (pos, tok) = self.advance_with_pos();
        trace!("advance => pos={}, tok={:?}", pos, tok);
        tok
    }

    #[inline]
    fn advance_with_pos(&mut self) -> (usize, Token) {
        match self.token_queue.pop_front() {
            Some((pos, tok)) => {
                self.offset = pos;
                (pos, tok)
            }
            None => (self.offset, Token::Eof),
        }
    }

    #[inline]
    fn peek(&self, lookahead: usize) -> &Token {
        self.token_queue
            .get(lookahead)
            .map(|(_pos, t)| t)
            .unwrap_or(&self.eof_token)
    }

    /// 根据当前 token 返回格式化错误
    fn err(&self, current_token: &Token, error_msg: &str, is_peek: bool) -> JmespathError {
        let mut actual_pos = self.offset;
        let mut buff = error_msg.to_string();
        buff.push_str(&format!(" -- found {:?}", current_token));
        if is_peek {
            if let Some((p, _)) = self.token_queue.get(0) {
                actual_pos = *p;
            }
        }
        JmespathError::new(self.expr, actual_pos, ErrorReason::Parse(buff))
    }

    /// Pratt 解析器主函数：根据右结合力（rbp）解析表达式
    fn expr(&mut self, rbp: usize) -> ParseResult {
        trace!("expr: start with rbp={}", rbp);
        let mut left = self.nud();
        while rbp < self.peek(0).lbp() {
            let left_ast = left?;
            trace!("expr: left_ast={:?}, next token => {:?}", left_ast, self.peek(0));
            left = self.led(Box::new(left_ast));
        }
        left
    }

    fn nud(&mut self) -> ParseResult {
        let (offset, token) = self.advance_with_pos();
        trace!("nud: token={:?} at offset={}", token, offset);
        match token {
            Token::At => {
                trace!("nud => Ast::Identity");
                Ok(Ast::Identity { offset })
            }
            Token::Identifier(value) => {
                trace!("nud => Ast::Field({:?})", value);
                Ok(Ast::Field { name: value, offset })
            }
            Token::QuotedIdentifier(value) => {
                trace!("nud => QuotedIdentifier={:?}", value);
                match self.peek(0) {
                    Token::Lparen => {
                        Err(self.err(&Token::Lparen, "Quoted strings can't be a function name", true))
                    }
                    _ => Ok(Ast::Field { name: value, offset }),
                }
            }
            Token::Star => {
                trace!("nud => '*' wildcard, using Identity as lhs");
                self.parse_wildcard_values(Box::new(Ast::Identity { offset }))
            }
            Token::Literal(value) => {
                trace!("nud => Ast::Literal: {:?}", value);
                Ok(Ast::Literal { value, offset })
            }
            Token::Lbracket => {
                trace!("nud => '[' start");
                match self.peek(0) {
                    Token::Number(_) | Token::Colon => self.parse_index(),
                    Token::Star if self.peek(1) == &Token::Rbracket => {
                        self.advance(); // consume '*'
                        self.parse_wildcard_index(Box::new(Ast::Identity { offset }))
                    }
                    _ => self.parse_multi_list(),
                }
            }
            Token::Flatten => {
                trace!("nud => Flatten token");
                self.parse_flatten(Box::new(Ast::Identity { offset }))
            }
            Token::Lbrace => {
                trace!("nud => object start, parsing multi-hash");
                let mut pairs = vec![];
                if let Token::Rbrace = self.peek(0) {
                    self.advance(); // consume '}'
                    return Ok(Ast::MultiHash { elements: pairs, offset });
                }
                loop {
                    pairs.push(self.parse_kvp()?);
                    match self.advance() {
                        Token::Rbrace => break,
                        Token::Comma => continue,
                        t => return Err(self.err(&t, "Expected '}' or ','", false)),
                    }
                }
                Ok(Ast::MultiHash { elements: pairs, offset })
            }
            t @ Token::Ampersand => {
                trace!("nud => '&' token => parse expref");
                let rhs = self.expr(t.lbp())?;
                Ok(Ast::Expref { ast: Box::new(rhs), offset })
            }
            t @ Token::Not => {
                trace!("nud => '!' token => parse Not");
                let node = self.expr(t.lbp())?;
                Ok(Ast::Not { node: Box::new(node), offset })
            }
            Token::Filter => {
                trace!("nud => Filter token");
                self.parse_filter(Box::new(Ast::Identity { offset }))
            }
            Token::Lparen => {
                trace!("nud => '(' token => parse subexpression");
                let result = self.expr(0)?;
                match self.advance() {
                    Token::Rparen => Ok(result),
                    tk => Err(self.err(&tk, "Expected ')' to close '('", false)),
                }
            }
            tk => {
                trace!("nud => Unexpected token: {:?}", tk);
                Err(self.err(&tk, "Unexpected nud token", false))
            }
        }
    }

    fn led(&mut self, left: Box<Ast>) -> ParseResult {
        let (offset, token) = self.advance_with_pos();
        trace!("led: token={:?} at offset={}, left={:?}", token, offset, left);
        match token {
            t @ Token::Dot => {
                trace!("led => Dot operator");
                if self.peek(0) == &Token::Star {
                    self.advance();
                    self.parse_wildcard_values(left)
                } else {
                    let rhs = self.parse_dot(t.lbp())?;
                    Ok(Ast::Subexpr { offset, lhs: left, rhs: Box::new(rhs) })
                }
            }
            Token::Lbracket => {
                trace!("led => '[' operator");
                if matches!(self.peek(0), Token::Number(_) | Token::Colon) {
                    Ok(Ast::Subexpr { offset, lhs: left, rhs: Box::new(self.parse_index()?) })
                } else {
                    self.advance(); // consume token (如 '*' 或 filter)
                    self.parse_wildcard_index(left)
                }
            }
            t @ Token::Or => {
                trace!("led => Or operator");
                let rhs = self.expr(t.lbp())?;
                Ok(Ast::Or { offset, lhs: left, rhs: Box::new(rhs) })
            }
            t @ Token::And => {
                trace!("led => And operator");
                let rhs = self.expr(t.lbp())?;
                Ok(Ast::And { offset, lhs: left, rhs: Box::new(rhs) })
            }
            t @ Token::Pipe => {
                trace!("led => Pipe operator");
                let rhs = self.expr(t.lbp())?;
                Ok(Ast::Subexpr { offset, lhs: left, rhs: Box::new(rhs) })
            }
            Token::Lparen => {
                trace!("led => Function call");
                match *left {
                    Ast::Field { ref name, .. } => {
                        let args = self.parse_list(Token::Rparen)?;
                        Ok(Ast::Function { offset, name: name.clone(), args })
                    }
                    _ => Err(self.err(self.peek(0), "Invalid function call (LHS is not a field)", true)),
                }
            }
            Token::Flatten => self.parse_flatten(left),
            Token::Filter => self.parse_filter(left),
            Token::Eq => self.parse_comparator(Comparator::Equal, left),
            Token::Ne => self.parse_comparator(Comparator::NotEqual, left),
            Token::Gt => self.parse_comparator(Comparator::GreaterThan, left),
            Token::Gte => self.parse_comparator(Comparator::GreaterThanEqual, left),
            Token::Lt => self.parse_comparator(Comparator::LessThan, left),
            Token::Lte => self.parse_comparator(Comparator::LessThanEqual, left),
            tk => Err(self.err(&tk, "Unexpected led token", false)),
        }
    }

    fn parse_kvp(&mut self) -> Result<KeyValuePair, JmespathError> {
        trace!("parse_kvp: parsing key");
        match self.advance() {
            Token::Identifier(value) | Token::QuotedIdentifier(value) => {
                if self.peek(0) == &Token::Colon {
                    self.advance();
                    let val = self.expr(0)?;
                    Ok(KeyValuePair { key: value, value: val })
                } else {
                    Err(self.err(self.peek(0), "Expected ':' after key", true))
                }
            }
            tk => Err(self.err(&tk, "Expected a key (identifier) in object", false)),
        }
    }

    /// 解析过滤器表达式：[? <expr> ]
    // fn parse_filter(&mut self, lhs: Box<Ast>) -> ParseResult {
    //     trace!("parse_filter: start");
    //     let condition_lhs = Box::new(self.expr(0)?);
    //     match self.advance() {
    //         Token::Rbracket => {
    //             let condition_rhs = Box::new(self.projection_rhs(Token::Filter.lbp())?);
    //             trace!("parse_filter: building filter with predicate={:?}", condition_lhs);
    //             Ok(Ast::Projection {
    //                 offset: self.offset,
    //                 lhs,
    //                 rhs: Box::new(Ast::Condition {
    //                     offset: self.offset,
    //                     predicate: condition_lhs,
    //                     then: condition_rhs,
    //                 }),
    //             })
    //         }
    //         tk => Err(self.err(&tk, "Expected ']' after filter condition", false)),
    //     }
    // }
    fn parse_filter(&mut self, lhs: Box<Ast>) -> ParseResult {
        trace!("parse_filter: parse condition expr inside '[? ... ]'");
        let condition_lhs = Box::new(self.expr(0)?);
        // 期待']'
        match self.advance() {
            Token::Rbracket => {
                // 默认 then 分支返回当前项自身
                let condition_rhs = Box::new(Ast::Identity { offset: self.offset });
                trace!("parse_filter => build Ast::Condition => predicate={:?}", condition_lhs);
                Ok(Ast::Projection {
                    offset: self.offset,
                    lhs,
                    rhs: Box::new(Ast::Condition {
                        offset: self.offset,
                        predicate: condition_lhs,
                        then: condition_rhs,
                    }),
                })
            }
            tk => Err(self.err(&tk, "Expected ']' after filter condition", false)),
        }
    }


    fn parse_flatten(&mut self, lhs: Box<Ast>) -> ParseResult {
        let rhs = Box::new(self.projection_rhs(Token::Flatten.lbp())?);
        trace!("parse_flatten: building flatten projection");
        Ok(Ast::Projection {
            offset: self.offset,
            lhs: Box::new(Ast::Flatten { offset: self.offset, node: lhs }),
            rhs,
        })
    }

    fn parse_comparator(&mut self, cmp: Comparator, lhs: Box<Ast>) -> ParseResult {
        trace!("parse_comparator: {:?}", cmp);
        let rhs = Box::new(self.expr(Token::Eq.lbp())?);
        Ok(Ast::Comparison { offset: self.offset, comparator: cmp, lhs, rhs })
    }

    fn parse_dot(&mut self, lbp: usize) -> ParseResult {
        trace!("parse_dot: handling '.'");
        if let Token::Lbracket = self.peek(0) {
            self.advance();
            self.parse_multi_list()
        } else {
            self.expr(lbp)
        }
    }

    fn projection_rhs(&mut self, lbp: usize) -> ParseResult {
        trace!("projection_rhs: peek={:?}", self.peek(0));
        match self.peek(0) {
            Token::Dot => {
                self.advance();
                self.parse_dot(lbp)
            }
            Token::Lbracket | Token::Filter => self.expr(lbp),
            t if t.lbp() < PROJECTION_STOP => Ok(Ast::Identity { offset: self.offset }),
            t => Err(self.err(t, "Expected '.', '[', or '[?'", true)),
        }
    }

    fn parse_wildcard_index(&mut self, lhs: Box<Ast>) -> ParseResult {
        match self.advance() {
            Token::Rbracket => {
                trace!("parse_wildcard_index: building projection for [*]");
                let rhs = Box::new(self.projection_rhs(Token::Star.lbp())?);
                Ok(Ast::Projection { offset: self.offset, lhs, rhs })
            }
            tk => Err(self.err(&tk, "Expected ']' for wildcard index", false)),
        }
    }

    fn parse_wildcard_values(&mut self, lhs: Box<Ast>) -> ParseResult {
        trace!("parse_wildcard_values: building object values projection");
        let rhs = Box::new(self.projection_rhs(Token::Star.lbp())?);
        Ok(Ast::Projection {
            offset: self.offset,
            lhs: Box::new(Ast::ObjectValues { offset: self.offset, node: lhs }),
            rhs,
        })
    }

    /// 解析索引表达式：例如 [0], [1:], [::-1] 等
    fn parse_index(&mut self) -> ParseResult {
        trace!("parse_index: start");
        let mut parts = [None, None, None]; // 分别表示 start, stop, step
        let mut pos = 0;
        loop {
            match self.advance() {
                Token::Number(value) => {
                    trace!("parse_index: got number {}", value);
                    parts[pos] = Some(value);
                    match self.peek(0) {
                        Token::Colon | Token::Rbracket => {}
                        t => return Err(self.err(t, "Expected ':' or ']'", true)),
                    };
                }
                Token::Rbracket => break,
                Token::Colon if pos >= 2 => {
                    return Err(self.err(&Token::Colon, "Too many colons in slice expr", false));
                }
                Token::Colon => {
                    pos += 1;
                    match self.peek(0) {
                        Token::Number(_) | Token::Colon | Token::Rbracket => {}
                        t => return Err(self.err(t, "Expected number, ':' or ']'", true)),
                    }
                }
                t => return Err(self.err(&t, "Expected number, ':', or ']'", false)),
            }
        }
        if pos == 0 {
            let idx_val = parts[0].ok_or_else(|| {
                JmespathError::new(
                    self.expr,
                    self.offset,
                    ErrorReason::Parse("Expected index number, found None".to_string()),
                )
            })?;
            trace!("parse_index: single index {}", idx_val);
            Ok(Ast::Index { offset: self.offset, idx: idx_val })
        } else {
            let step = parts[2].unwrap_or(1);
            trace!(
                "parse_index: slice with start={:?}, stop={:?}, step={}",
                parts[0],
                parts[1],
                step
            );
            Ok(Ast::Projection {
                offset: self.offset,
                lhs: Box::new(Ast::Slice {
                    offset: self.offset,
                    start: parts[0],
                    stop: parts[1],
                    step,
                }),
                rhs: Box::new(self.projection_rhs(Token::Star.lbp())?),
            })
        }
    }

    /// 解析多元素列表： [ expr, expr, ... ]
    fn parse_multi_list(&mut self) -> ParseResult {
        let start_offset = self.offset;
        trace!("parse_multi_list: start");
        let elements = self.parse_list(Token::Rbracket)?;
        if elements.len() == 1 {
            if let Ast::Literal { value, offset } = &elements[0] {
                if let Some(s) = value.as_string() {
                    trace!("parse_multi_list: single literal detected, treating as field: {}", s);
                    return Ok(Ast::Field { offset: *offset, name: s.clone() });
                }
            }
        }
        Ok(Ast::MultiList { offset: start_offset, elements })
    }

    /// 解析由逗号分隔的列表，直到遇到指定的闭合 token
    fn parse_list(&mut self, closing: Token) -> Result<Vec<Ast>, JmespathError> {
        trace!("parse_list: start, expecting closing token {:?}", closing);
        let mut nodes = vec![];
        while self.peek(0) != &closing {
            let expr0 = self.expr(0)?;
            nodes.push(expr0);
            if self.peek(0) == &Token::Comma {
                self.advance();
                if self.peek(0) == &closing {
                    return Err(self.err(self.peek(0), "invalid trailing comma", true));
                }
            }
        }
        self.advance(); // 消耗闭合 token
        trace!("parse_list: done, parsed {} elements", nodes.len());
        Ok(nodes)
    }
}