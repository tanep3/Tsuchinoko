//! Parser module - pest-based Python parser

mod ast;

pub use ast::*;

use pest::Parser;
use pest_derive::Parser;
use crate::error::TsuchinokoError;

#[derive(Parser)]
#[grammar = "parser/python.pest"]
pub struct PythonParser;

/// Parse Python source code into AST
pub fn parse(source: &str) -> Result<Program, TsuchinokoError> {
    // For Phase 1, we'll parse simple single-line statements
    let mut statements = Vec::new();
    
    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        if let Some(stmt) = parse_line(line, line_num + 1)? {
            statements.push(stmt);
        }
    }
    
    Ok(Program { statements })
}

/// Parse a single line of Python code
fn parse_line(line: &str, line_num: usize) -> Result<Option<Stmt>, TsuchinokoError> {
    // Try to parse as assignment: x: int = 10 or x = 10
    if let Some(stmt) = try_parse_assignment(line, line_num)? {
        return Ok(Some(stmt));
    }
    
    // Try to parse as return statement
    if line.starts_with("return") {
        return Ok(Some(parse_return(line, line_num)?));
    }
    
    // For now, skip other statements
    Ok(None)
}

/// Try to parse an assignment statement
fn try_parse_assignment(line: &str, line_num: usize) -> Result<Option<Stmt>, TsuchinokoError> {
    // Pattern: name: type = value  or  name = value
    if !line.contains('=') || line.contains("==") {
        return Ok(None);
    }
    
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Ok(None);
    }
    
    let left = parts[0].trim();
    let right = parts[1].trim();
    
    // Check for type annotation
    let (name, type_hint) = if left.contains(':') {
        let left_parts: Vec<&str> = left.splitn(2, ':').collect();
        let name = left_parts[0].trim();
        let type_str = left_parts[1].trim();
        (name, Some(parse_type_hint(type_str)?))
    } else {
        (left, None)
    };
    
    // Parse the value expression
    let value = parse_expr(right, line_num)?;
    
    Ok(Some(Stmt::Assign {
        target: name.to_string(),
        type_hint,
        value,
    }))
}

/// Parse a return statement
fn parse_return(line: &str, line_num: usize) -> Result<Stmt, TsuchinokoError> {
    let value_str = line.strip_prefix("return").unwrap().trim();
    
    if value_str.is_empty() {
        Ok(Stmt::Return(None))
    } else {
        let expr = parse_expr(value_str, line_num)?;
        Ok(Stmt::Return(Some(expr)))
    }
}

/// Parse a type hint
fn parse_type_hint(type_str: &str) -> Result<TypeHint, TsuchinokoError> {
    // Handle generic types like list[int]
    if let Some(bracket_pos) = type_str.find('[') {
        let name = type_str[..bracket_pos].trim();
        let params_str = &type_str[bracket_pos + 1..type_str.len() - 1];
        let params: Result<Vec<_>, _> = params_str
            .split(',')
            .map(|s| parse_type_hint(s.trim()))
            .collect();
        
        Ok(TypeHint {
            name: name.to_string(),
            params: params?,
        })
    } else {
        Ok(TypeHint {
            name: type_str.to_string(),
            params: vec![],
        })
    }
}

/// Parse an expression
fn parse_expr(expr_str: &str, line_num: usize) -> Result<Expr, TsuchinokoError> {
    let expr_str = expr_str.trim();
    
    // Try to parse as integer
    if let Ok(n) = expr_str.parse::<i64>() {
        return Ok(Expr::IntLiteral(n));
    }
    
    // Try to parse as float
    if let Ok(f) = expr_str.parse::<f64>() {
        return Ok(Expr::FloatLiteral(f));
    }
    
    // Try to parse as boolean
    if expr_str == "True" {
        return Ok(Expr::BoolLiteral(true));
    }
    if expr_str == "False" {
        return Ok(Expr::BoolLiteral(false));
    }
    
    // Try to parse as None
    if expr_str == "None" {
        return Ok(Expr::NoneLiteral);
    }
    
    // Try to parse as string literal
    if (expr_str.starts_with('"') && expr_str.ends_with('"')) ||
       (expr_str.starts_with('\'') && expr_str.ends_with('\'')) {
        let s = &expr_str[1..expr_str.len() - 1];
        return Ok(Expr::StringLiteral(s.to_string()));
    }
    
    // Try to parse as list literal
    if expr_str.starts_with('[') && expr_str.ends_with(']') {
        let inner = &expr_str[1..expr_str.len() - 1];
        if inner.is_empty() {
            return Ok(Expr::List(vec![]));
        }
        let elements: Result<Vec<_>, _> = inner
            .split(',')
            .map(|s| parse_expr(s.trim(), line_num))
            .collect();
        return Ok(Expr::List(elements?));
    }
    
    // Try to parse as binary operation (simple cases)
    for (op_str, op) in [
        (" + ", BinOp::Add),
        (" - ", BinOp::Sub),
        (" * ", BinOp::Mul),
        (" / ", BinOp::Div),
        (" == ", BinOp::Eq),
        (" != ", BinOp::NotEq),
        (" > ", BinOp::Gt),
        (" < ", BinOp::Lt),
        (" >= ", BinOp::GtEq),
        (" <= ", BinOp::LtEq),
    ] {
        if let Some(pos) = expr_str.find(op_str) {
            let left = parse_expr(&expr_str[..pos], line_num)?;
            let right = parse_expr(&expr_str[pos + op_str.len()..], line_num)?;
            return Ok(Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }
    }
    
    // Try to parse as function call: func(args)
    if let Some(paren_pos) = expr_str.find('(') {
        if expr_str.ends_with(')') {
            let func_name = &expr_str[..paren_pos];
            let args_str = &expr_str[paren_pos + 1..expr_str.len() - 1];
            
            let args = if args_str.is_empty() {
                vec![]
            } else {
                args_str
                    .split(',')
                    .map(|s| parse_expr(s.trim(), line_num))
                    .collect::<Result<Vec<_>, _>>()?
            };
            
            return Ok(Expr::Call {
                func: func_name.to_string(),
                args,
            });
        }
    }
    
    // Assume it's an identifier
    if expr_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(Expr::Ident(expr_str.to_string()));
    }
    
    Err(TsuchinokoError::ParseError {
        line: line_num,
        message: format!("Cannot parse expression: {}", expr_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_assignment() {
        let result = parse("x: int = 10").unwrap();
        assert_eq!(result.statements.len(), 1);
        if let Stmt::Assign { target, type_hint, value } = &result.statements[0] {
            assert_eq!(target, "x");
            assert!(type_hint.is_some());
            assert_eq!(type_hint.as_ref().unwrap().name, "int");
            assert_eq!(*value, Expr::IntLiteral(10));
        } else {
            panic!("Expected Assign statement");
        }
    }

    #[test]
    fn test_parse_list_type() {
        let result = parse("nums: list[int] = [1, 2, 3]").unwrap();
        if let Stmt::Assign { type_hint, value, .. } = &result.statements[0] {
            let th = type_hint.as_ref().unwrap();
            assert_eq!(th.name, "list");
            assert_eq!(th.params[0].name, "int");
            if let Expr::List(elements) = value {
                assert_eq!(elements.len(), 3);
            }
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let result = parse("result: int = a + b").unwrap();
        if let Stmt::Assign { value, .. } = &result.statements[0] {
            if let Expr::BinOp { op, .. } = value {
                assert_eq!(*op, BinOp::Add);
            }
        }
    }

    #[test]
    fn test_parse_function_call() {
        let result = parse("x = print(hello)").unwrap();
        if let Stmt::Assign { value, .. } = &result.statements[0] {
            if let Expr::Call { func, args } = value {
                assert_eq!(func, "print");
                assert_eq!(args.len(), 1);
            }
        }
    }
}
