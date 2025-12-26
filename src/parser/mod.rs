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
    let lines: Vec<&str> = source.lines().collect();
    let mut statements = Vec::new();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }
        
        // Try to parse function definition
        if line.starts_with("def ") {
            let (stmt, consumed) = parse_function_def(&lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        // Try to parse if statement
        if line.starts_with("if ") {
            let (stmt, consumed) = parse_if_stmt(&lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        // Try to parse for loop
        if line.starts_with("for ") {
            let (stmt, consumed) = parse_for_stmt(&lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        // Try to parse while loop
        if line.starts_with("while ") {
            let (stmt, consumed) = parse_while_stmt(&lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        // Try to parse single-line statement
        if let Some(stmt) = parse_line(line, i + 1)? {
            statements.push(stmt);
        }
        i += 1;
    }
    
    Ok(Program { statements })
}

/// Parse a function definition
fn parse_function_def(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;
    
    // Parse: def func_name(params) -> return_type:
    let def_part = line.strip_prefix("def ").unwrap();
    let colon_pos = def_part.rfind(':').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing colon in function definition".to_string(),
    })?;
    
    let signature = &def_part[..colon_pos];
    
    // Parse function name and parameters
    let paren_start = signature.find('(').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing opening parenthesis".to_string(),
    })?;
    let paren_end = signature.rfind(')').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing closing parenthesis".to_string(),
    })?;
    
    let name = signature[..paren_start].trim().to_string();
    let params_str = &signature[paren_start + 1..paren_end];
    
    // Parse parameters
    let params = if params_str.trim().is_empty() {
        vec![]
    } else {
        params_str
            .split(',')
            .map(|p| parse_param(p.trim(), line_num))
            .collect::<Result<Vec<_>, _>>()?
    };
    
    // Parse return type
    let return_type = if let Some(arrow_pos) = signature.find("->") {
        let type_str = signature[arrow_pos + 2..].trim().trim_end_matches(')');
        Some(parse_type_hint(type_str)?)
    } else {
        None
    };
    
    // Parse body (indented block)
    let (body, consumed) = parse_block(lines, start + 1)?;
    
    Ok((
        Stmt::FuncDef {
            name,
            params,
            return_type,
            body,
        },
        consumed + 1,
    ))
}

/// Parse an if statement
fn parse_if_stmt(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;
    
    // Parse: if condition:
    let if_part = line.strip_prefix("if ").unwrap();
    let colon_pos = if_part.rfind(':').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing colon in if statement".to_string(),
    })?;
    
    let condition_str = &if_part[..colon_pos];
    let condition = parse_expr(condition_str.trim(), line_num)?;
    
    // Parse then body
    let (then_body, then_consumed) = parse_block(lines, start + 1)?;
    let mut total_consumed = then_consumed + 1;
    
    // Check for elif and else clauses
    let mut elif_clauses = Vec::new();
    let mut else_body = None;
    
    let mut next_line_idx = start + total_consumed;
    
    // Parse elif clauses
    while next_line_idx < lines.len() {
        let next_line = lines[next_line_idx].trim();
        if next_line.starts_with("elif ") {
            let elif_part = next_line.strip_prefix("elif ").unwrap();
            let colon_pos = elif_part.rfind(':').ok_or_else(|| TsuchinokoError::ParseError {
                line: next_line_idx + 1,
                message: "Missing colon in elif".to_string(),
            })?;
            let elif_cond = parse_expr(&elif_part[..colon_pos], next_line_idx + 1)?;
            let (elif_body, elif_consumed) = parse_block(lines, next_line_idx + 1)?;
            elif_clauses.push((elif_cond, elif_body));
            total_consumed += elif_consumed + 1;
            next_line_idx += elif_consumed + 1;
        } else {
            break;
        }
    }
    
    // Parse else clause
    if next_line_idx < lines.len() {
        let next_line = lines[next_line_idx].trim();
        if next_line == "else:" || next_line.starts_with("else:") {
            let (else_blk, else_consumed) = parse_block(lines, next_line_idx + 1)?;
            else_body = Some(else_blk);
            total_consumed += else_consumed + 1;
        }
    }
    
    Ok((
        Stmt::If {
            condition,
            then_body,
            elif_clauses,
            else_body,
        },
        total_consumed,
    ))
}

/// Parse a for loop
fn parse_for_stmt(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;
    
    // Parse: for var in iterable:
    let for_part = line.strip_prefix("for ").unwrap();
    let colon_pos = for_part.rfind(':').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing colon in for loop".to_string(),
    })?;
    
    let loop_part = &for_part[..colon_pos];
    let in_pos = loop_part.find(" in ").ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing 'in' in for loop".to_string(),
    })?;
    
    let target = loop_part[..in_pos].trim().to_string();
    let iter_str = loop_part[in_pos + 4..].trim();
    let iter = parse_expr(iter_str, line_num)?;
    
    let (body, consumed) = parse_block(lines, start + 1)?;
    
    Ok((
        Stmt::For {
            target,
            iter,
            body,
        },
        consumed + 1,
    ))
}

/// Parse a while loop
fn parse_while_stmt(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;
    
    // Parse: while condition:
    let while_part = line.strip_prefix("while ").unwrap();
    let colon_pos = while_part.rfind(':').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing colon in while loop".to_string(),
    })?;
    
    let condition = parse_expr(&while_part[..colon_pos], line_num)?;
    let (body, consumed) = parse_block(lines, start + 1)?;
    
    Ok((
        Stmt::While {
            condition,
            body,
        },
        consumed + 1,
    ))
}

/// Parse an indented block
fn parse_block(lines: &[&str], start: usize) -> Result<(Vec<Stmt>, usize), TsuchinokoError> {
    let mut statements = Vec::new();
    let mut i = start;
    
    // Determine the indentation level
    if i >= lines.len() {
        return Ok((statements, 0));
    }
    
    let first_line = lines[i];
    let indent_level = first_line.len() - first_line.trim_start().len();
    
    // If no indentation, empty block
    if indent_level == 0 && !first_line.trim().is_empty() {
        return Ok((statements, 0));
    }
    
    while i < lines.len() {
        let line = lines[i];
        let line_trim = line.trim();
        
        // Skip empty lines within block
        if line_trim.is_empty() {
            i += 1;
            continue;
        }
        
        // Check indentation
        let current_indent = line.len() - line.trim_start().len();
        if current_indent < indent_level {
            break;
        }
        
        // Parse nested structures
        if line_trim.starts_with("if ") {
            let (stmt, consumed) = parse_if_stmt(lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        if line_trim.starts_with("for ") {
            let (stmt, consumed) = parse_for_stmt(lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        if line_trim.starts_with("while ") {
            let (stmt, consumed) = parse_while_stmt(lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }
        
        // Parse single-line statement
        if let Some(stmt) = parse_line(line_trim, i + 1)? {
            statements.push(stmt);
        }
        i += 1;
    }
    
    Ok((statements, i - start))
}

/// Parse a function parameter
fn parse_param(param_str: &str, _line_num: usize) -> Result<Param, TsuchinokoError> {
    if let Some(colon_pos) = param_str.find(':') {
        let name = param_str[..colon_pos].trim().to_string();
        let type_str = param_str[colon_pos + 1..].trim();
        Ok(Param {
            name,
            type_hint: Some(parse_type_hint(type_str)?),
        })
    } else {
        Ok(Param {
            name: param_str.to_string(),
            type_hint: None,
        })
    }
}

/// Parse a single line of Python code
fn parse_line(line: &str, line_num: usize) -> Result<Option<Stmt>, TsuchinokoError> {
    let line = line.trim();
    
    // Skip pass statement
    if line == "pass" {
        return Ok(None);
    }
    
    // Try to parse as assignment
    if let Some(stmt) = try_parse_assignment(line, line_num)? {
        return Ok(Some(stmt));
    }
    
    // Try to parse as return statement
    if line.starts_with("return") {
        return Ok(Some(parse_return(line, line_num)?));
    }
    
    // Try to parse as expression statement
    if let Ok(expr) = parse_expr(line, line_num) {
        return Ok(Some(Stmt::Expr(expr)));
    }
    
    Ok(None)
}

/// Try to parse an assignment statement
fn try_parse_assignment(line: &str, line_num: usize) -> Result<Option<Stmt>, TsuchinokoError> {
    if !line.contains('=') || line.contains("==") || line.contains("!=") || 
       line.contains("<=") || line.contains(">=") {
        return Ok(None);
    }
    
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Ok(None);
    }
    
    let left = parts[0].trim();
    let right = parts[1].trim();
    
    // Check for tuple unpacking: a, b = func()
    let left_parts = split_by_comma_balanced(left);
    if left_parts.len() > 1 {
        // Validation: Ensure all targets are simple identifiers
        let mut targets = Vec::new();
        for part in left_parts {
            if part.chars().all(|c| c.is_alphanumeric() || c == '_') {
                targets.push(part);
            } else {
                return Err(TsuchinokoError::ParseError {
                    line: line_num,
                    message: format!("Invalid unpacking target: {}", part),
                });
            }
        }
        
        let value = parse_expr(right, line_num)?;
        return Ok(Some(Stmt::TupleAssign {
            targets,
            value,
        }));
    }
    
    // Check for index assignment: arr[i] = val
    if left.ends_with(']') {
        if let Some(bracket_pos) = find_matching_bracket(left, 0, '[', ']') {
            if bracket_pos == left.len() - 1 {
                // Determine start of index bracket
                // We need to support nested like arr[i][j], so search for last open bracket that matches the end
                // However, our parse_expr handles precedence.
                // Let's parse the left side as an expression first.
                if let Ok(Expr::Index { target, index }) = parse_expr(left, line_num) {
                    let value = parse_expr(right, line_num)?;
                    return Ok(Some(Stmt::IndexAssign {
                        target: *target,
                        index: *index,
                        value,
                    }));
                }
            }
        }
    }
    
    // Normal assignment: name: type = val  or  name = val
    // Check for type annotation
    let (name, type_hint) = if left.contains(':') {
        let left_parts: Vec<&str> = left.splitn(2, ':').collect();
        let name = left_parts[0].trim();
        let type_str = left_parts[1].trim();
        (name, Some(parse_type_hint(type_str)?))
    } else {
        (left, None)
    };
    
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
    let type_str = type_str.trim();
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
    
    if expr_str.is_empty() {
        return Err(TsuchinokoError::ParseError {
            line: line_num,
            message: "Empty expression".to_string(),
        });
    }
    
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
    
    // Try to parse as parenthesized expression
    if expr_str.starts_with('(') && expr_str.ends_with(')') {
        let inner = &expr_str[1..expr_str.len() - 1];
        return parse_expr(inner, line_num);
    }
    
    // Try to parse as list literal or comprehension
    if expr_str.starts_with('[') && find_matching_bracket(expr_str, 0, '[', ']') == Some(expr_str.len() - 1) {
        let inner = &expr_str[1..expr_str.len() - 1];
        if inner.is_empty() {
            return Ok(Expr::List(vec![]));
        }
        
        // Check if it is a comprehension: expression for target in iter
        // We look for " for " keyword
        if let Some(for_pos) = find_keyword_balanced(inner, "for") {
            let left_part = &inner[..for_pos];
            let right_part = &inner[for_pos + 3..]; // skip "for"
            
            // In right_part, we need " in "
            if let Some(in_pos) = find_keyword_balanced(right_part, "in") {
                let target_str = &right_part[..in_pos].trim();
                let iter_str = &right_part[in_pos + 2..].trim(); // skip "in"
                
                let elt = parse_expr(left_part.trim(), line_num)?;
                let iter = parse_expr(iter_str, line_num)?;
                
                return Ok(Expr::ListComp {
                    elt: Box::new(elt),
                    target: target_str.to_string(),
                    iter: Box::new(iter),
                });
            }
        }
        
        let elements = split_by_comma_balanced(inner);
        let parsed: Result<Vec<_>, _> = elements
            .iter()
            .map(|s| parse_expr(s.trim(), line_num))
            .collect();
        return Ok(Expr::List(parsed?));
    }
    
    // Try to parse as dict literal {key: value, ...}
    if expr_str.starts_with('{') && find_matching_bracket(expr_str, 0, '{', '}') == Some(expr_str.len() - 1) {
        let inner = &expr_str[1..expr_str.len() - 1];
        if inner.is_empty() {
            return Ok(Expr::Dict(vec![]));
        }
        
        let entries = split_by_comma_balanced(inner);
        let mut parsed_entries = Vec::new();
        for entry in entries {
            let entry = entry.trim();
            // Find the colon that separates key: value
            if let Some(colon_pos) = entry.find(':') {
                let key = parse_expr(entry[..colon_pos].trim(), line_num)?;
                let value = parse_expr(entry[colon_pos + 1..].trim(), line_num)?;
                parsed_entries.push((key, value));
            } else {
                return Err(TsuchinokoError::ParseError {
                    line: line_num,
                    message: format!("Invalid dict entry: {}", entry),
                });
            }
        }
        return Ok(Expr::Dict(parsed_entries));
    }
    
    // Try to parse as binary operation (lowest precedence first)
    for (op_str, op) in [
        (" or ", BinOp::Or),
        (" and ", BinOp::And),
    ] {
        if let Some(pos) = find_operator_balanced(expr_str, op_str) {
            let left = parse_expr(&expr_str[..pos], line_num)?;
            let right = parse_expr(&expr_str[pos + op_str.len()..], line_num)?;
            return Ok(Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }
    }
    
    // Comparison operators
    for (op_str, op) in [
        (" == ", BinOp::Eq),
        (" != ", BinOp::NotEq),
        (" >= ", BinOp::GtEq),
        (" <= ", BinOp::LtEq),
        (" > ", BinOp::Gt),
        (" < ", BinOp::Lt),
        (" in ", BinOp::In),
    ] {
        if let Some(pos) = find_operator_balanced(expr_str, op_str) {
            let left = parse_expr(&expr_str[..pos], line_num)?;
            let right = parse_expr(&expr_str[pos + op_str.len()..], line_num)?;
            return Ok(Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }
    }
    
    // Additive operators (left to right, find rightmost)
    for (op_str, op) in [
        (" + ", BinOp::Add),
        (" - ", BinOp::Sub),
    ] {
        if let Some(pos) = find_operator_balanced_rtl(expr_str, op_str) {
            let left = parse_expr(&expr_str[..pos], line_num)?;
            let right = parse_expr(&expr_str[pos + op_str.len()..], line_num)?;
            return Ok(Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }
    }
    
    // Multiplicative operators
    for (op_str, op) in [
        (" * ", BinOp::Mul),
        (" // ", BinOp::FloorDiv),
        (" / ", BinOp::Div),
        (" % ", BinOp::Mod),
    ] {
        if let Some(pos) = find_operator_balanced_rtl(expr_str, op_str) {
            let left = parse_expr(&expr_str[..pos], line_num)?;
            let right = parse_expr(&expr_str[pos + op_str.len()..], line_num)?;
            return Ok(Expr::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }
    }
    
    // Power operator (right to left)
    if let Some(pos) = find_operator_balanced(expr_str, " ** ") {
        let left = parse_expr(&expr_str[..pos], line_num)?;
        let right = parse_expr(&expr_str[pos + 4..], line_num)?;
        return Ok(Expr::BinOp {
            left: Box::new(left),
            op: BinOp::Pow,
            right: Box::new(right),
        });
    }
    
    // Try to parse as Call (ends with ')')
    if expr_str.ends_with(')') {
        if let Some(open_pos) = find_matching_bracket_rtl(expr_str, expr_str.len() - 1, ')', '(') {
             let func_part = &expr_str[..open_pos];
             let args_part = &expr_str[open_pos + 1..expr_str.len() - 1];
             
             // If func_part is not empty, it is a call. If empty, it is (expr) handled by loops above?
             // Actually remove_parens handles (trimmed).
             // If we have " (a) ", remove_parens strips it.
             // If we have "func(a)", func_part="func".
             if !func_part.trim().is_empty() {
                 let args = if args_part.trim().is_empty() {
                     vec![]
                 } else {
                     let arg_parts = split_by_comma_balanced(args_part);
                     arg_parts.iter().map(|s| parse_expr(s.trim(), line_num)).collect::<Result<Vec<_>,_>>()?
                 };
                 
                 return Ok(Expr::Call {
                     func: Box::new(parse_expr(func_part, line_num)?),
                     args
                 });
             }
        }
    }

    // Try to parse as Index (ends with ']')
    if expr_str.ends_with(']') {
        if let Some(open_pos) = find_matching_bracket_rtl(expr_str, expr_str.len() - 1, ']', '[') {
            let target_part = &expr_str[..open_pos];
            let index_part = &expr_str[open_pos + 1..expr_str.len() - 1];
            
            // If target_part is empty, it's a list literal or comprehension -> handled below.
            if !target_part.trim().is_empty() {
                return Ok(Expr::Index {
                    target: Box::new(parse_expr(target_part, line_num)?),
                    index: Box::new(parse_expr(index_part, line_num)?),
                });
            }
        }
    }
    
    // Try to parse as Attribute (contains '.')
    // Find the right-most dot that is not in parens/brackets/strings
    if let Some(dot_pos) = find_char_balanced_rtl(expr_str, '.') {
         let right = &expr_str[dot_pos+1..];
         // Check if right side looks like an attribute identifier
         if right.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && 
            right.chars().all(|c| c.is_alphanumeric() || c == '_') {
             
             return Ok(Expr::Attribute {
                 value: Box::new(parse_expr(&expr_str[..dot_pos], line_num)?),
                 attr: right.to_string(),
             });
         }
    }
    
    // Try to parse as tuple (comma-separated values without brackets)
    let parts = split_by_comma_balanced(expr_str);
    if parts.len() > 1 {
        let elements: Result<Vec<_>, _> = parts
            .iter()
            .map(|s| parse_expr(s.trim(), line_num))
            .collect();
        return Ok(Expr::Tuple(elements?));
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

/// Find matching bracket starting from a position
/// Find matching opening bracket starting from a position (moving Left)
fn find_matching_bracket_rtl(s: &str, start: usize, close_char: char, open_char: char) -> Option<usize> {
    let mut depth = 0;
    
    for (i, c) in s.char_indices().rev() {
        if i > start {
            continue;
        }
        
        if c == close_char {
            depth += 1;
        } else if c == open_char {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Find matching bracket starting from a position (Left to Right)
fn find_matching_bracket(s: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s[start..].char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(start + i);
            }
        }
    }
    None
}

/// Find last occurrence of a char respecting bracket balance
fn find_char_balanced_rtl(s: &str, target: char) -> Option<usize> {
    let mut depth_paren = 0;
    let mut depth_bracket = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    
    let chars: Vec<char> = s.chars().collect();
    
    for i in (0..chars.len()).rev() {
        let c = chars[i];
        
        if in_string {
            if c == string_char {
                in_string = false;
            }
            continue;
        }
        
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
             _ if depth_paren == 0 && depth_bracket == 0 => {
                 if c == target {
                     return Some(i);
                 }
            }
            _ => {}
        }
    }
    None
}

/// Split string by comma, respecting bracket balance
fn split_by_comma_balanced(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth_paren = 0;
    let mut depth_bracket = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    
    for c in s.chars() {
        if in_string {
            current.push(c);
            if c == string_char {
                in_string = false;
            }
            continue;
        }
        
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
                current.push(c);
            }
            '(' => {
                depth_paren += 1;
                current.push(c);
            }
            ')' => {
                depth_paren -= 1;
                current.push(c);
            }
            '[' => {
                depth_bracket += 1;
                current.push(c);
            }
            ']' => {
                depth_bracket -= 1;
                current.push(c);
            }
            ',' if depth_paren == 0 && depth_bracket == 0 => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    
    parts
}

/// Find operator position respecting bracket balance (left to right)
fn find_operator_balanced(s: &str, op: &str) -> Option<usize> {
    let mut depth_paren = 0;
    let mut depth_bracket = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    
    let chars: Vec<char> = s.chars().collect();
    let op_chars: Vec<char> = op.chars().collect();
    
    for i in 0..chars.len() {
        let c = chars[i];
        
        if in_string {
            if c == string_char {
                in_string = false;
            }
            continue;
        }
        
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            _ if depth_paren == 0 && depth_bracket == 0 => {
                if i + op_chars.len() <= chars.len() {
                    let slice: String = chars[i..i + op_chars.len()].iter().collect();
                    if slice == op {
                        return Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    
    None
}

/// Find operator position respecting bracket balance (right to left)
fn find_operator_balanced_rtl(s: &str, op: &str) -> Option<usize> {
    let mut depth_paren = 0;
    let mut depth_bracket = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    
    let chars: Vec<char> = s.chars().collect();
    let op_chars: Vec<char> = op.chars().collect();
    let mut last_found: Option<usize> = None;
    
    for i in 0..chars.len() {
        let c = chars[i];
        
        if in_string {
            if c == string_char {
                in_string = false;
            }
            continue;
        }
        
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            _ if depth_paren == 0 && depth_bracket == 0 => {
                if i + op_chars.len() <= chars.len() {
                    let slice: String = chars[i..i + op_chars.len()].iter().collect();
                    if slice == op {
                        last_found = Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    
    last_found
}

/// Find keyword position respecting bracket balance (left to right)
/// The keyword must be surrounded by non-alphanumeric characters (or start/end of string)
fn find_keyword_balanced(s: &str, keyword: &str) -> Option<usize> {
    let mut depth_paren = 0;
    let mut depth_bracket = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    
    let chars: Vec<char> = s.chars().collect();
    let keyword_chars: Vec<char> = keyword.chars().collect();
    
    for i in 0..chars.len() {
        let c = chars[i];
        
        if in_string {
            if c == string_char {
                in_string = false;
            }
            continue;
        }
        
        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c;
            }
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            _ if depth_paren == 0 && depth_bracket == 0 => {
                // Check if keyword matches here
                if i + keyword_chars.len() <= chars.len() {
                    let slice: String = chars[i..i + keyword_chars.len()].iter().collect();
                    if slice == keyword {
                        // Check word boundaries
                        let prev_char = if i > 0 { Some(chars[i - 1]) } else { None };
                        let next_char = if i + keyword_chars.len() < chars.len() {
                            Some(chars[i + keyword_chars.len()])
                        } else {
                            None
                        };
                        
                        let is_start_ok = prev_char.map_or(true, |c| !c.is_alphanumeric() && c != '_');
                        let is_end_ok = next_char.map_or(true, |c| !c.is_alphanumeric() && c != '_');
                        
                        if is_start_ok && is_end_ok {
                            return Some(i);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    None
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
            assert_eq!(*value, Expr::IntLiteral(10));
        }
    }

    #[test]
    fn test_parse_function_def() {
        let code = r#"
def add(a: int, b: int) -> int:
    return a + b
"#;
        let result = parse(code).unwrap();
        assert_eq!(result.statements.len(), 1);
        if let Stmt::FuncDef { name, params, return_type, body } = &result.statements[0] {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert!(return_type.is_some());
            assert_eq!(body.len(), 1);
        }
    }

    #[test]
    fn test_parse_if_stmt() {
        let code = r#"
if x > 0:
    y = 1
else:
    y = 0
"#;
        let result = parse(code).unwrap();
        assert_eq!(result.statements.len(), 1);
        if let Stmt::If { then_body, else_body, .. } = &result.statements[0] {
            assert_eq!(then_body.len(), 1);
            assert!(else_body.is_some());
        }
    }

    #[test]
    fn test_parse_for_loop() {
        let code = r#"
for i in range(10):
    print(i)
"#;
        let result = parse(code).unwrap();
        assert_eq!(result.statements.len(), 1);
        if let Stmt::For { target, body, .. } = &result.statements[0] {
            assert_eq!(target, "i");
            assert_eq!(body.len(), 1);
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let code = r#"
while x > 0:
    x = x - 1
"#;
        let result = parse(code).unwrap();
        assert_eq!(result.statements.len(), 1);
        if let Stmt::While { body, .. } = &result.statements[0] {
            assert_eq!(body.len(), 1);
        }
    }
}
