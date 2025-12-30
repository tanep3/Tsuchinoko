//! Parser module - pest-based Python parser

mod ast;
mod utils;

pub use ast::*;

use utils::{
    find_char_balanced, find_char_balanced_rtl, find_keyword_balanced, find_matching_bracket,
    find_matching_bracket_rtl, find_operator_balanced, find_operator_balanced_rtl,
    split_by_comma_balanced,
};

use crate::error::TsuchinokoError;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/python.pest"]
pub struct PythonParser;

/// Preprocess source to join lines with unclosed brackets/parens
fn preprocess_multiline(source: &str) -> String {
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut bracket_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_string = false;
    let mut string_char = ' ';

    for line in source.lines() {
        // Count brackets in the line
        for c in line.chars() {
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
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
        }

        if current_line.is_empty() {
            current_line = line.to_string();
        } else {
            // Append this line to current, preserving a single space
            current_line.push(' ');
            current_line.push_str(line.trim());
        }

        // If all brackets are closed, emit the line
        if bracket_depth == 0 && paren_depth == 0 && brace_depth == 0 {
            result.push(current_line);
            current_line = String::new();
        }
    }

    // Don't forget the last line if any
    if !current_line.is_empty() {
        result.push(current_line);
    }

    result.join("\n")
}

/// Parse Python source code into AST
pub fn parse(source: &str) -> Result<Program, TsuchinokoError> {
    // Preprocess: join lines with unclosed brackets
    let preprocessed = preprocess_multiline(source);
    let lines: Vec<&str> = preprocessed.lines().collect();
    let mut statements = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        // Try to parse class definition (with optional @dataclass decorator)
        if line.starts_with("@") || line.starts_with("class ") {
            let (stmt, consumed) = parse_class_def(&lines, i)?;
            statements.push(stmt);
            i += consumed;
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

        // Try to parse try-except
        if line.starts_with("try:") || line == "try:" {
            let (stmt, consumed) = parse_try_stmt(&lines, i)?;
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

/// Parse a try-except statement
fn parse_try_stmt(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    // First line should be "try:"
    let line_num = start + 1;

    // Parse try body
    let (try_body, try_consumed) = parse_block(lines, start + 1)?;

    // Find except clause
    let except_start = start + 1 + try_consumed;
    if except_start >= lines.len() {
        return Err(TsuchinokoError::ParseError {
            line: line_num,
            message: "Expected 'except' after try block".to_string(),
        });
    }

    let except_line = lines[except_start].trim();
    if !except_line.starts_with("except") {
        return Err(TsuchinokoError::ParseError {
            line: except_start + 1,
            message: format!("Expected 'except', got: {except_line}"),
        });
    }

    // Parse exception type (optional)
    let except_type = if except_line.starts_with("except ") && except_line.contains(':') {
        let colon_pos = except_line.find(':').unwrap();
        let type_str = except_line[7..colon_pos].trim();
        if type_str.is_empty() {
            None
        } else {
            Some(type_str.to_string())
        }
    } else {
        None
    };

    // Parse except body
    let (except_body, except_consumed) = parse_block(lines, except_start + 1)?;

    let total_consumed = 1 + try_consumed + 1 + except_consumed;

    Ok((
        Stmt::TryExcept {
            try_body,
            except_type,
            except_body,
        },
        total_consumed,
    ))
}

/// Parse a class definition (@dataclass class Name: ...)
fn parse_class_def(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let mut i = start;
    let line_num = start + 1;

    // Skip @dataclass decorator if present
    let line = lines[i].trim();
    if line.starts_with('@') {
        // Just skip the decorator line for now
        i += 1;
        if i >= lines.len() {
            return Err(TsuchinokoError::ParseError {
                line: line_num,
                message: "Expected class definition after decorator".to_string(),
            });
        }
    }

    let class_line = lines[i].trim();
    let line_num = i + 1;

    // Parse: class ClassName:
    if !class_line.starts_with("class ") {
        return Err(TsuchinokoError::ParseError {
            line: line_num,
            message: format!("Expected 'class' keyword, got: {class_line}"),
        });
    }

    let class_part = class_line.strip_prefix("class ").unwrap();
    let colon_pos = class_part
        .rfind(':')
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing colon in class definition".to_string(),
        })?;

    let name = class_part[..colon_pos].trim().to_string();

    // Parse class body (fields and methods)
    let (fields, methods, body_consumed) = parse_class_body(lines, i + 1)?;

    let consumed = (i - start) + 1 + body_consumed;

    Ok((
        Stmt::ClassDef {
            name,
            fields,
            methods,
        },
        consumed,
    ))
}

/// Parse class body (field definitions and methods)
fn parse_class_body(
    lines: &[&str],
    start: usize,
) -> Result<(Vec<Field>, Vec<MethodDef>, usize), TsuchinokoError> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let mut i = start;

    if i >= lines.len() {
        return Err(TsuchinokoError::ParseError {
            line: start + 1,
            message: "Expected class body".to_string(),
        });
    }

    // Determine expected indentation level
    let first_line = lines[i];
    let indent_level = first_line.len() - first_line.trim_start().len();

    while i < lines.len() {
        let line = lines[i];
        let line_trim = line.trim();

        // Skip empty lines
        if line_trim.is_empty() {
            i += 1;
            continue;
        }

        // Skip comment lines
        if line_trim.starts_with('#') {
            i += 1;
            continue;
        }

        // Check indentation
        let current_indent = line.len() - line.trim_start().len();
        if current_indent < indent_level {
            // Dedent - end of class body
            break;
        }

        // Check for @staticmethod decorator
        if line_trim == "@staticmethod" {
            // Next line should be a method def
            i += 1;
            if i >= lines.len() {
                return Err(TsuchinokoError::ParseError {
                    line: i,
                    message: "Expected method after @staticmethod".to_string(),
                });
            }
            let method_line = lines[i].trim();
            if method_line.starts_with("def ") {
                let (method, consumed) = parse_method_def(lines, i, true)?;
                methods.push(method);
                i += consumed;
            } else {
                return Err(TsuchinokoError::ParseError {
                    line: i + 1,
                    message: "Expected method definition after @staticmethod".to_string(),
                });
            }
            continue;
        }

        // Check for method definition
        if line_trim.starts_with("def ") {
            let (method, consumed) = parse_method_def(lines, i, false)?;

            // Extract fields from __init__ method
            if method.name == "__init__" {
                let init_fields = extract_fields_from_init(&method.body, &method.params);
                fields.extend(init_fields);
            }

            methods.push(method);
            i += consumed;
            continue;
        }
        
        // Skip docstrings (""" or ''')
        if line_trim.starts_with("\"\"\"") || line_trim.starts_with("'''") {
            // Single-line docstring: """..."""
            if (line_trim.starts_with("\"\"\"") && line_trim.ends_with("\"\"\"") && line_trim.len() > 6)
                || (line_trim.starts_with("'''") && line_trim.ends_with("'''") && line_trim.len() > 6)
            {
                i += 1;
                continue;
            }
            // Multi-line docstring
            let end_marker = if line_trim.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
            i += 1;
            while i < lines.len() && !lines[i].trim().ends_with(end_marker) {
                i += 1;
            }
            i += 1; // Skip the closing """
            continue;
        }

        // Parse field: field_name: type (for dataclass style)
        let line_num = i + 1;
        if let Some(colon_pos) = line_trim.find(':') {
            let field_name = line_trim[..colon_pos].trim().to_string();
            let type_str = line_trim[colon_pos + 1..].trim();

            // Handle field with default value (x: int = 0)
            let type_str = if let Some(eq_pos) = type_str.find('=') {
                type_str[..eq_pos].trim()
            } else {
                type_str
            };

            // Skip empty type (like in method params)
            if !type_str.is_empty() {
                let type_hint = parse_type_hint(type_str)?;
                fields.push(Field {
                    name: field_name,
                    type_hint,
                });
            }
        } else {
            return Err(TsuchinokoError::ParseError {
                line: line_num,
                message: format!("Expected field or method definition: {line_trim}"),
            });
        }

        i += 1;
    }

    Ok((fields, methods, i - start))
}

/// Parse a method definition (def method_name(self, ...): ...)
fn parse_method_def(
    lines: &[&str],
    start: usize,
    is_static: bool,
) -> Result<(MethodDef, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;

    // Parse: def method_name(params) -> ret:
    if !line.starts_with("def ") {
        return Err(TsuchinokoError::ParseError {
            line: line_num,
            message: format!("Expected 'def', got: {line}"),
        });
    }

    let rest = line.strip_prefix("def ").unwrap();
    let paren_start = rest.find('(').ok_or_else(|| TsuchinokoError::ParseError {
        line: line_num,
        message: "Missing '(' in method definition".to_string(),
    })?;

    let name = rest[..paren_start].trim().to_string();

    // Find matching closing paren
    let paren_end = find_closing_paren(rest, paren_start)?;
    let params_str = &rest[paren_start + 1..paren_end];

    // Parse parameters (skip 'self' for instance methods)
    let mut params = Vec::new();
    for param_str in split_params(params_str) {
        let param_str = param_str.trim();
        if param_str.is_empty() || param_str == "self" {
            continue;
        }

        let param = if let Some(colon_pos) = param_str.find(':') {
            let param_name = param_str[..colon_pos].trim().to_string();
            let type_str = param_str[colon_pos + 1..].trim();
            // Handle default value
            let type_str = if let Some(eq_pos) = type_str.find('=') {
                type_str[..eq_pos].trim()
            } else {
                type_str
            };
            Param {
                name: param_name,
                type_hint: Some(parse_type_hint(type_str)?),
                default: None,
                variadic: false,
            }
        } else {
            Param {
                name: param_str.to_string(),
                type_hint: None,
                default: None,
                variadic: false,
            }
        };
        params.push(param);
    }

    // Parse return type
    let after_params = &rest[paren_end + 1..];
    let return_type = if let Some(arrow_pos) = after_params.find("->") {
        let ret_str = after_params[arrow_pos + 2..].trim();
        let ret_str = ret_str.strip_suffix(':').unwrap_or(ret_str).trim();
        if !ret_str.is_empty() {
            Some(parse_type_hint(ret_str)?)
        } else {
            None
        }
    } else {
        None
    };

    // Parse method body
    let (body, body_consumed) = parse_block(lines, start + 1)?;

    Ok((
        MethodDef {
            name,
            params,
            return_type,
            body,
            is_static,
        },
        1 + body_consumed,
    ))
}

/// Extract fields from __init__ body by looking for self.field = value patterns
fn extract_fields_from_init(body: &[Stmt], params: &[Param]) -> Vec<Field> {
    let mut fields = Vec::new();

    // Build a map of parameter names to their types
    let param_types: std::collections::HashMap<&str, Option<&TypeHint>> = params
        .iter()
        .map(|p| (p.name.as_str(), p.type_hint.as_ref()))
        .collect();

    for stmt in body {
        // Look for self.__field = value or self.field = value
        if let Stmt::Assign {
            target,
            type_hint,
            value,
        } = stmt
        {
            if target.starts_with("self.") {
                let field_name = target.strip_prefix("self.").unwrap();
                // Convert __private to private (strip leading __)
                let field_name = if field_name.starts_with("__") && !field_name.ends_with("__") {
                    field_name.strip_prefix("__").unwrap()
                } else {
                    field_name
                };

                // Determine type: explicit type hint > infer from param in expr > Any
                let hint = if let Some(h) = type_hint.clone() {
                    h
                } else {
                    // Try to find any parameter reference in the RHS expression
                    infer_type_from_expr(value, &param_types)
                };

                fields.push(Field {
                    name: field_name.to_string(),
                    type_hint: hint,
                });
            }
        }
    }

    fields
}

/// Recursively find parameter references in an expression and return the first matching type
fn infer_type_from_expr(
    expr: &Expr,
    param_types: &std::collections::HashMap<&str, Option<&TypeHint>>,
) -> TypeHint {
    match expr {
        // Direct identifier - check if it's a parameter
        Expr::Ident(name) => {
            if let Some(Some(hint)) = param_types.get(name.as_str()) {
                (*hint).clone()
            } else {
                TypeHint {
                    name: "Any".to_string(),
                    params: vec![],
                }
            }
        }
        // Function call - check arguments for parameters
        Expr::Call { func, args, .. } => {
            // First check if any arg contains a param
            for arg in args {
                let hint = infer_type_from_expr(arg, param_types);
                if hint.name != "Any" {
                    return hint;
                }
            }
            // Then check the function expression itself
            infer_type_from_expr(func, param_types)
        }
        // Attribute access - check the target (e.g., param.items())
        Expr::Attribute { value, .. } => infer_type_from_expr(value, param_types),
        // Binary operation - check both sides
        Expr::BinOp { left, right, .. } => {
            let left_hint = infer_type_from_expr(left, param_types);
            if left_hint.name != "Any" {
                return left_hint;
            }
            infer_type_from_expr(right, param_types)
        }
        // Other expression types - return Any
        _ => TypeHint {
            name: "Any".to_string(),
            params: vec![],
        },
    }
}

/// Split method parameters by comma, respecting nested brackets
fn split_params(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < s.len() {
        result.push(&s[start..]);
    }

    result
}

/// Find closing parenthesis matching opening at given position
fn find_closing_paren(s: &str, open_pos: usize) -> Result<usize, TsuchinokoError> {
    let mut depth = 0;
    for (i, c) in s[open_pos..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(open_pos + i);
                }
            }
            _ => {}
        }
    }
    Err(TsuchinokoError::ParseError {
        line: 0,
        message: "Unmatched parenthesis".to_string(),
    })
}

/// Parse a function definition
fn parse_function_def(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;

    // Parse: def func_name(params) -> return_type:
    let def_part = line.strip_prefix("def ").unwrap();
    let colon_pos = def_part
        .rfind(':')
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing colon in function definition".to_string(),
        })?;

    let signature = &def_part[..colon_pos];

    // Parse function name and parameters
    let paren_start = signature
        .find('(')
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing opening parenthesis".to_string(),
        })?;
    let paren_end = signature
        .rfind(')')
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing closing parenthesis".to_string(),
        })?;

    let name = signature[..paren_start].trim().to_string();
    let params_str = &signature[paren_start + 1..paren_end];

    // Parse parameters (use balanced split for nested brackets like Callable[[int], int])
    let params = if params_str.trim().is_empty() {
        vec![]
    } else {
        split_by_comma_balanced(params_str)
            .iter()
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
    let colon_pos = if_part
        .rfind(':')
        .ok_or_else(|| TsuchinokoError::ParseError {
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
            let colon_pos = elif_part
                .rfind(':')
                .ok_or_else(|| TsuchinokoError::ParseError {
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
    let colon_pos = for_part
        .rfind(':')
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing colon in for loop".to_string(),
        })?;

    let loop_part = &for_part[..colon_pos];
    let in_pos = loop_part
        .find(" in ")
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing 'in' in for loop".to_string(),
        })?;

    let target = loop_part[..in_pos].trim().to_string();
    let iter_str = loop_part[in_pos + 4..].trim();
    let iter = parse_expr(iter_str, line_num)?;

    let (body, consumed) = parse_block(lines, start + 1)?;

    Ok((Stmt::For { target, iter, body }, consumed + 1))
}

/// Parse a while loop
fn parse_while_stmt(lines: &[&str], start: usize) -> Result<(Stmt, usize), TsuchinokoError> {
    let line = lines[start].trim();
    let line_num = start + 1;

    // Parse: while condition:
    let while_part = line.strip_prefix("while ").unwrap();
    let colon_pos = while_part
        .rfind(':')
        .ok_or_else(|| TsuchinokoError::ParseError {
            line: line_num,
            message: "Missing colon in while loop".to_string(),
        })?;

    let condition = parse_expr(&while_part[..colon_pos], line_num)?;
    let (body, consumed) = parse_block(lines, start + 1)?;

    Ok((Stmt::While { condition, body }, consumed + 1))
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
        let _line_trim = line.trim();

        // Skip empty lines within block
        if line_trim.is_empty() {
            i += 1;
            continue;
        }

        // Skip comment lines
        if line_trim.starts_with('#') {
            i += 1;
            continue;
        }

        // Skip comment lines
        if line_trim.starts_with('#') {
            i += 1;
            continue;
        }

        // Check indentation
        let current_indent = line.len() - line.trim_start().len();
        if current_indent < indent_level {
            break;
        }

        // Parse nested structures
        if line_trim.starts_with("def ") {
            let (stmt, consumed) = parse_function_def(lines, i)?;
            statements.push(stmt);
            i += consumed;
            continue;
        }

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

        // Parse try-except
        if line_trim.starts_with("try:") || line_trim == "try:" {
            let (stmt, consumed) = parse_try_stmt(lines, i)?;
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

/// Parse a function parameter (supports default values: param: type = default, and variadic *args)
fn parse_param(param_str: &str, line_num: usize) -> Result<Param, TsuchinokoError> {
    let param_str = param_str.trim();

    // Check for variadic parameter (*args or *args: type)
    if param_str.starts_with('*') && !param_str.starts_with("**") {
        let rest = param_str[1..].trim();
        // Check if there's a type hint (*args: int)
        if let Some(colon_pos) = rest.find(':') {
            let name = rest[..colon_pos].trim().to_string();
            let type_str = rest[colon_pos + 1..].trim();
            return Ok(Param {
                name,
                type_hint: Some(parse_type_hint(type_str)?),
                default: None,
                variadic: true,
            });
        } else {
            return Ok(Param {
                name: rest.to_string(),
                type_hint: None,
                default: None,
                variadic: true,
            });
        }
    }

    // Check for default value first (param: type = default or param = default)
    // Find = that is not == or !=
    let eq_pos = find_char_balanced(param_str, '=');

    if let Some(eq_idx) = eq_pos {
        // Make sure it's not == or !=
        let before = if eq_idx > 0 {
            param_str.chars().nth(eq_idx - 1)
        } else {
            None
        };
        let after = param_str.chars().nth(eq_idx + 1);

        if before != Some('=')
            && before != Some('!')
            && before != Some('<')
            && before != Some('>')
            && after != Some('=')
        {
            // This is a default value assignment
            let left_part = param_str[..eq_idx].trim();
            let default_str = param_str[eq_idx + 1..].trim();
            let default_expr = parse_expr(default_str, line_num)?;

            // Parse left part (name: type or just name)
            if let Some(colon_pos) = left_part.find(':') {
                let name = left_part[..colon_pos].trim().to_string();
                let type_str = left_part[colon_pos + 1..].trim();
                return Ok(Param {
                    name,
                    type_hint: Some(parse_type_hint(type_str)?),
                    default: Some(default_expr),
                    variadic: false,
                });
            } else {
                return Ok(Param {
                    name: left_part.to_string(),
                    type_hint: None,
                    default: Some(default_expr),
                    variadic: false,
                });
            }
        }
    }

    // No default value
    if let Some(colon_pos) = param_str.find(':') {
        let name = param_str[..colon_pos].trim().to_string();
        let type_str = param_str[colon_pos + 1..].trim();
        Ok(Param {
            name,
            type_hint: Some(parse_type_hint(type_str)?),
            default: None,
            variadic: false,
        })
    } else {
        Ok(Param {
            name: param_str.trim().to_string(),
            type_hint: None,
            default: None,
            variadic: false,
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

    // Try to parse as augmented assignment (+=, -=, *=, /=, //=, %=)
    // Must check before regular assignment
    for (op_str, aug_op) in [
        ("//=", AugAssignOp::FloorDiv), // Check //= before /=
        ("+=", AugAssignOp::Add),
        ("-=", AugAssignOp::Sub),
        ("*=", AugAssignOp::Mul),
        ("/=", AugAssignOp::Div),
        ("%=", AugAssignOp::Mod),
    ] {
        if let Some(op_pos) = line.find(op_str) {
            let target = line[..op_pos].trim();
            let value_str = line[op_pos + op_str.len()..].trim();

            if target.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let value = parse_expr(value_str, line_num)?;
                return Ok(Some(Stmt::AugAssign {
                    target: target.to_string(),
                    op: aug_op,
                    value,
                }));
            }
        }
    }

    // Try to parse as assignment
    if let Some(stmt) = try_parse_assignment(line, line_num)? {
        return Ok(Some(stmt));
    }

    // Try to parse as break statement
    if line == "break" {
        return Ok(Some(Stmt::Break));
    }

    // Try to parse as continue statement
    if line == "continue" {
        return Ok(Some(Stmt::Continue));
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
    // Find assignment operator '=' respecting quotes and brackets
    // We need a helper for LTR balanced search
    let eq_pos = match find_char_balanced(line, '=') {
        Some(pos) => pos,
        None => return Ok(None),
    };

    // Ensure it's not '==', '!=', '<=', '>='
    let bytes = line.as_bytes();
    if eq_pos > 0 {
        let prev = bytes[eq_pos - 1] as char;
        if prev == '!' || prev == '<' || prev == '>' || prev == '=' {
            return Ok(None);
        }
    }
    if eq_pos + 1 < bytes.len() {
        let next = bytes[eq_pos + 1] as char;
        if next == '=' {
            return Ok(None);
        }
    }

    let left = line[..eq_pos].trim();
    let right_full = line[eq_pos + 1..].trim();
    // Strip inline comments (# that is not inside a string)
    let right = if let Some(hash_pos) = find_char_balanced(right_full, '#') {
        right_full[..hash_pos].trim()
    } else {
        right_full
    };

    // Check for tuple unpacking: a, b = func()
    // Also support index swap: a[i], a[j] = a[j], a[i]
    let left_parts = split_by_comma_balanced(left);
    if left_parts.len() > 1 {
        // Check if all targets are either identifiers, index expressions, or starred (*var)
        let mut has_index_target = false;
        let mut _has_starred_target = false;
        for part in &left_parts {
            let part_trimmed = part.trim();
            if part_trimmed
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
            {
                // Simple identifier - OK
            } else if part_trimmed.starts_with('*')
                && part_trimmed[1..]
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
            {
                // Starred identifier (*tail) - OK
                _has_starred_target = true;
            } else if part_trimmed.ends_with(']') {
                // Index expression - also OK
                has_index_target = true;
            } else {
                return Err(TsuchinokoError::ParseError {
                    line: line_num,
                    message: format!("Invalid unpacking target: {part}"),
                });
            }
        }

        // If we have index targets (swap pattern), generate IndexSwap statement
        if has_index_target {
            // Parse as: left1, left2 = right1, right2 -> swap assignments
            let left_exprs: Result<Vec<_>, _> = left_parts
                .iter()
                .map(|s| parse_expr(s.trim(), line_num))
                .collect();
            let left_exprs = left_exprs?;

            let right_parts = split_by_comma_balanced(right);
            let right_exprs: Result<Vec<_>, _> = right_parts
                .iter()
                .map(|s| parse_expr(s.trim(), line_num))
                .collect();
            let right_exprs = right_exprs?;

            return Ok(Some(Stmt::IndexSwap {
                left_targets: left_exprs,
                right_values: right_exprs,
            }));
        }

        // Simple tuple assign - detect starred targets
        let mut starred_index: Option<usize> = None;
        let targets: Vec<String> = left_parts
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let trimmed = s.trim();
                if trimmed.starts_with('*') {
                    starred_index = Some(i);
                    trimmed[1..].to_string() // Remove * prefix
                } else {
                    trimmed.to_string()
                }
            })
            .collect();

        let value = parse_expr(right, line_num)?;
        return Ok(Some(Stmt::TupleAssign { targets, value, starred_index }));
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

    // Check for attribute assignment: self.__field = value or self.field = value
    // This must be handled before type annotation check since it contains ':'
    if left.starts_with("self.") && !left.contains('[') {
        // Parse the type hint if present: self.__field: Type = value
        // First, try to find ':' that's part of a type hint (not in the attribute name)
        let (attr_name, type_hint) = if let Some(colon_pos) = left[5..].find(':') {
            let actual_colon_pos = 5 + colon_pos;
            let name = left[..actual_colon_pos].trim();
            let type_str = left[actual_colon_pos + 1..].trim();
            (name, Some(parse_type_hint(type_str)?))
        } else {
            (left, None)
        };

        let value = parse_expr(right, line_num)?;
        return Ok(Some(Stmt::Assign {
            target: attr_name.to_string(), // Keep full "self.__field" format
            type_hint,
            value,
        }));
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

    // Handle forward reference string literals like 'Numbers' -> Numbers
    let type_str = if (type_str.starts_with('\'') && type_str.ends_with('\''))
        || (type_str.starts_with('"') && type_str.ends_with('"'))
    {
        &type_str[1..type_str.len() - 1]
    } else {
        type_str
    };

    // Special case: [int, int] (bare list literal for Callable params)
    // This represents a tuple of types, not a list type
    if type_str.starts_with('[') && type_str.ends_with(']') {
        let inner = &type_str[1..type_str.len() - 1];
        let param_strs = split_by_comma_balanced(inner);
        let params: Result<Vec<_>, _> = param_strs
            .iter()
            .map(|s| parse_type_hint(s.trim()))
            .collect();

        // Return as "__param_list__" which we'll handle specially in from_python_hint
        return Ok(TypeHint {
            name: "__param_list__".to_string(),
            params: params?,
        });
    }

    // Find the first '[' that starts type parameters (not nested)
    if let Some(bracket_pos) = type_str.find('[') {
        // Find matching closing bracket
        let closing_pos = find_matching_bracket(type_str, bracket_pos, '[', ']');
        if closing_pos.is_none() {
            return Ok(TypeHint {
                name: type_str.to_string(),
                params: vec![],
            });
        }

        let name = type_str[..bracket_pos].trim();
        let params_str = &type_str[bracket_pos + 1..type_str.len() - 1];

        // Use balanced comma split for nested brackets like Callable[[int, int], bool]
        let param_strs = split_by_comma_balanced(params_str);
        let params: Result<Vec<_>, _> = param_strs
            .iter()
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

/// Parse an f-string literal (content without the f"" quotes)
/// f"{x}: {y}" -> parts: ["", ": ", ""], values: [x, y]
fn parse_fstring(content: &str, line_num: usize) -> Result<Expr, TsuchinokoError> {
    let mut parts = Vec::new();
    let mut values = Vec::new();
    let mut current_part = String::new();
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            // Check for escaped brace {{
            if chars.peek() == Some(&'{') {
                chars.next();
                current_part.push('{');
                continue;
            }

            // Start of expression
            parts.push(current_part.clone());
            current_part = String::new();

            // Extract expression until closing brace
            let mut expr_str = String::new();
            let mut depth = 1;
            for c2 in chars.by_ref() {
                if c2 == '{' {
                    depth += 1;
                    expr_str.push(c2);
                } else if c2 == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    expr_str.push(c2);
                } else {
                    expr_str.push(c2);
                }
            }

            // Parse the expression inside {}
            let expr = parse_expr(&expr_str, line_num)?;
            values.push(expr);
        } else if c == '}' {
            // Check for escaped brace }}
            if chars.peek() == Some(&'}') {
                chars.next();
                current_part.push('}');
            } else {
                current_part.push(c);
            }
        } else {
            current_part.push(c);
        }
    }

    parts.push(current_part);

    Ok(Expr::FString { parts, values })
}

/// Find the first colon in a lambda expression that separates params from body
/// Handles nested brackets/parens correctly
fn find_lambda_colon(expr: &str) -> Option<usize> {
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';

    // Start searching after "lambda"
    for (i, c) in expr.char_indices() {
        if i < 6 {
            continue;
        } // Skip "lambda"

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
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            ':' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Some(i);
            }
            _ => {}
        }
    }

    None
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

    // Check for starred expression (*expr)
    if expr_str.starts_with('*') && !expr_str.starts_with("**") {
        let inner = &expr_str[1..];
        let inner_expr = parse_expr(inner, line_num)?;
        return Ok(Expr::Starred(Box::new(inner_expr)));
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

    // Try to parse as unary "not" operator
    if let Some(stripped) = expr_str.strip_prefix("not ") {
        let operand_str = stripped.trim();
        let operand = parse_expr(operand_str, line_num)?;
        return Ok(Expr::UnaryOp {
            op: UnaryOp::Not,
            operand: Box::new(operand),
        });
    }

    // Try to parse as lambda expression: lambda params: body
    if expr_str.starts_with("lambda") {
        // Find the colon that separates params from body
        // Need to handle nested lambdas carefully
        if let Some(colon_pos) = find_lambda_colon(expr_str) {
            let params_str = expr_str[6..colon_pos].trim(); // "lambda" is 6 chars
            let body_str = expr_str[colon_pos + 1..].trim();

            // Parse params (comma-separated identifiers, or empty for lambda:)
            let params: Vec<String> = if params_str.is_empty() {
                vec![]
            } else {
                params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            };

            // Parse body expression
            let body = parse_expr(body_str, line_num)?;

            return Ok(Expr::Lambda {
                params,
                body: Box::new(body),
            });
        }
    }

    // Try to parse as f-string (f"..." or f'...')
    if (expr_str.starts_with("f\"") && expr_str.ends_with('"'))
        || (expr_str.starts_with("f'") && expr_str.ends_with('\''))
    {
        return parse_fstring(&expr_str[2..expr_str.len() - 1], line_num);
    }

    // Try to parse as string literal
    if (expr_str.starts_with('"') && expr_str.ends_with('"'))
        || (expr_str.starts_with('\'') && expr_str.ends_with('\''))
    {
        let s = &expr_str[1..expr_str.len() - 1];
        return Ok(Expr::StringLiteral(s.to_string()));
    }

    // Try to parse as parenthesized expression
    if expr_str.starts_with('(') && expr_str.ends_with(')') {
        let inner = &expr_str[1..expr_str.len() - 1];
        return parse_expr(inner, line_num);
    }

    // Try to parse as list literal or comprehension
    if expr_str.starts_with('[')
        && find_matching_bracket(expr_str, 0, '[', ']') == Some(expr_str.len() - 1)
    {
        let inner = &expr_str[1..expr_str.len() - 1];
        if inner.is_empty() {
            return Ok(Expr::List(vec![]));
        }

        // Check if it is a comprehension: expression for target in iter [if condition]
        // We look for " for " keyword
        if let Some(for_pos) = find_keyword_balanced(inner, "for") {
            let left_part = &inner[..for_pos];
            let right_part = &inner[for_pos + 3..]; // skip "for"

            // In right_part, we need " in "
            if let Some(in_pos) = find_keyword_balanced(right_part, "in") {
                let target_str = &right_part[..in_pos].trim();
                let after_in = &right_part[in_pos + 2..].trim(); // skip "in"

                // Check for " if " condition
                let (iter_str, condition) =
                    if let Some(if_pos) = find_keyword_balanced(after_in, "if") {
                        let iter_part = &after_in[..if_pos].trim();
                        let cond_part = &after_in[if_pos + 2..].trim(); // skip "if"
                        let cond_expr = parse_expr(cond_part, line_num)?;
                        (iter_part.to_string(), Some(Box::new(cond_expr)))
                    } else {
                        (after_in.to_string(), None)
                    };

                let elt = parse_expr(left_part.trim(), line_num)?;
                let iter = parse_expr(&iter_str, line_num)?;

                return Ok(Expr::ListComp {
                    elt: Box::new(elt),
                    target: target_str.to_string(),
                    iter: Box::new(iter),
                    condition,
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
    if expr_str.starts_with('{')
        && find_matching_bracket(expr_str, 0, '{', '}') == Some(expr_str.len() - 1)
    {
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
                    message: format!("Invalid dict entry: {entry}"),
                });
            }
        }
        return Ok(Expr::Dict(parsed_entries));
    }

    // Check for "bare" generator expression: expression for target in iter [if condition]
    // This is for cases like tuple(x for x in y) or (x for x in y) where outer parens are stripped
    if let Some(for_pos) = find_keyword_balanced(expr_str, "for") {
        let left_part = &expr_str[..for_pos];
        let right_part = &expr_str[for_pos + 3..]; // skip "for"

        // In right_part, we need " in "
        if let Some(in_pos) = find_keyword_balanced(right_part, "in") {
            let target_str = &right_part[..in_pos].trim();
            let after_in = &right_part[in_pos + 2..].trim(); // skip "in"

            // Check for " if " condition
            let (iter_str, condition) = if let Some(if_pos) = find_keyword_balanced(after_in, "if")
            {
                let iter_part = &after_in[..if_pos].trim();
                let cond_part = &after_in[if_pos + 2..].trim(); // skip "if"
                let cond_expr = parse_expr(cond_part, line_num)?;
                (iter_part.to_string(), Some(Box::new(cond_expr)))
            } else {
                (after_in.to_string(), None)
            };

            let elt = parse_expr(left_part.trim(), line_num)?;
            let iter = parse_expr(&iter_str, line_num)?;

            return Ok(Expr::GenExpr {
                elt: Box::new(elt),
                target: target_str.to_string(),
                iter: Box::new(iter),
                condition,
            });
        }
    }

    // Conditional Expression: body if test else orelse
    // Priority is lower than binary ops, so check here (before binary ops).
    if let Some(if_pos) = find_keyword_balanced(expr_str, "if") {
        let after_if = &expr_str[if_pos + 2..];
        if let Some(else_pos) = find_keyword_balanced(after_if, "else") {
            // Found " if ... else ..." pattern
            let body_str = &expr_str[..if_pos];
            let test_str = &after_if[..else_pos];
            let orelse_str = &after_if[else_pos + 4..];

            let body = parse_expr(body_str.trim(), line_num)?;
            let test = parse_expr(test_str.trim(), line_num)?;
            let orelse = parse_expr(orelse_str.trim(), line_num)?;

            return Ok(Expr::IfExp {
                test: Box::new(test),
                body: Box::new(body),
                orelse: Box::new(orelse),
            });
        }
    }

    // Try to parse as binary operation (lowest precedence first)
    for (op_str, op) in [(" or ", BinOp::Or), (" and ", BinOp::And)] {
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

    // Comparison operators (check longer ones first)
    for (op_str, op) in [
        ("==", BinOp::Eq),
        ("!=", BinOp::NotEq),
        (">=", BinOp::GtEq),
        ("<=", BinOp::LtEq),
        (">", BinOp::Gt),
        ("<", BinOp::Lt),
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

    // "in" operator (keyword, needs spaces)
    if let Some(pos) = find_operator_balanced(expr_str, " in ") {
        let left = parse_expr(&expr_str[..pos], line_num)?;
        let right = parse_expr(&expr_str[pos + 4..], line_num)?;
        return Ok(Expr::BinOp {
            left: Box::new(left),
            op: BinOp::In,
            right: Box::new(right),
        });
    }

    // "is not" operator (must check before "is")
    if let Some(pos) = find_operator_balanced(expr_str, " is not ") {
        let left = parse_expr(&expr_str[..pos], line_num)?;
        let right = parse_expr(&expr_str[pos + 8..], line_num)?;
        return Ok(Expr::BinOp {
            left: Box::new(left),
            op: BinOp::IsNot,
            right: Box::new(right),
        });
    }

    // "is" operator
    if let Some(pos) = find_operator_balanced(expr_str, " is ") {
        let left = parse_expr(&expr_str[..pos], line_num)?;
        let right = parse_expr(&expr_str[pos + 4..], line_num)?;
        return Ok(Expr::BinOp {
            left: Box::new(left),
            op: BinOp::Is,
            right: Box::new(right),
        });
    }

    // Additive operators (left to right, find rightmost)
    for (op_str, op) in [("+", BinOp::Add), ("-", BinOp::Sub)] {
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
        ("//", BinOp::FloorDiv), // longest match first
        ("*", BinOp::Mul),
        ("/", BinOp::Div),
        ("%", BinOp::Mod),
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
    if let Some(pos) = find_operator_balanced(expr_str, "**") {
        let left = parse_expr(&expr_str[..pos], line_num)?;
        let right = parse_expr(&expr_str[pos + 2..], line_num)?;
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
                let (args, kwargs) = if args_part.trim().is_empty() {
                    (vec![], vec![])
                } else {
                    // Check if this is a generator expression being passed as a single argument
                    // e.g. join(x for x in y) or func(a for a in b if c)
                    // If we find a top-level "for" and NO top-level comma before it, it's a single gen expr.
                    let is_single_gen_expr =
                        if let Some(for_pos) = find_keyword_balanced(args_part, "for") {
                            let left_part = &args_part[..for_pos];
                            // Check if left_part has a comma
                            split_by_comma_balanced(left_part).len() == 1
                        } else {
                            false
                        };

                    let arg_parts = if is_single_gen_expr {
                        vec![args_part.to_string()]
                    } else {
                        split_by_comma_balanced(args_part)
                    };

                    // Separate positional args from keyword args
                    let mut positional_args = Vec::new();
                    let mut keyword_args = Vec::new();

                    for arg_str in arg_parts {
                        let arg_str = arg_str.trim();
                        // Check for keyword argument pattern: name=value (not ==)
                        // Must have = but not ==, and the left side must be a simple identifier
                        if let Some(eq_pos) = find_char_balanced(arg_str, '=') {
                            // Make sure it's not == or !=
                            let before = if eq_pos > 0 {
                                arg_str.chars().nth(eq_pos - 1)
                            } else {
                                None
                            };
                            let after = arg_str.chars().nth(eq_pos + 1);
                            if before != Some('=')
                                && before != Some('!')
                                && before != Some('<')
                                && before != Some('>')
                                && after != Some('=')
                            {
                                let name_part = arg_str[..eq_pos].trim();
                                let value_part = arg_str[eq_pos + 1..].trim();
                                // Check if name_part is a valid identifier (simple ident, no dots, brackets etc)
                                if !name_part.is_empty()
                                    && name_part.chars().all(|c| c.is_alphanumeric() || c == '_')
                                    && name_part
                                        .chars()
                                        .next()
                                        .map(|c| c.is_alphabetic() || c == '_')
                                        .unwrap_or(false)
                                {
                                    let value_expr = parse_expr(value_part, line_num)?;
                                    keyword_args.push((name_part.to_string(), value_expr));
                                    continue;
                                }
                            }
                        }
                        // Not a keyword argument, treat as positional
                        positional_args.push(parse_expr(arg_str, line_num)?);
                    }

                    (positional_args, keyword_args)
                };

                return Ok(Expr::Call {
                    func: Box::new(parse_expr(func_part, line_num)?),
                    args,
                    kwargs,
                });
            }
        }
    }

    // Try to parse as Index or Slice (ends with ']')
    if expr_str.ends_with(']') {
        if let Some(open_pos) = find_matching_bracket_rtl(expr_str, expr_str.len() - 1, ']', '[') {
            let target_part = &expr_str[..open_pos];
            let index_part = &expr_str[open_pos + 1..expr_str.len() - 1];

            // If target_part is empty, it's a list literal or comprehension -> handled below.
            // If target_part ends with comma, it's likely a tuple containing a list -> skip index parsing
            // e.g. "0, []" should be parsed as tuple, not index
            let target_trimmed = target_part.trim();
            if !target_trimmed.is_empty() && !target_trimmed.ends_with(',') {
                // Check if this is a slice (contains ':')
                if let Some(colon_pos) = find_char_balanced(index_part, ':') {
                    // It's a slice: target[start:end]
                    let start_str = &index_part[..colon_pos].trim();
                    let end_str = &index_part[colon_pos + 1..].trim();

                    let start = if start_str.is_empty() {
                        None
                    } else {
                        Some(Box::new(parse_expr(start_str, line_num)?))
                    };

                    let end = if end_str.is_empty() {
                        None
                    } else {
                        Some(Box::new(parse_expr(end_str, line_num)?))
                    };

                    return Ok(Expr::Slice {
                        target: Box::new(parse_expr(target_part, line_num)?),
                        start,
                        end,
                    });
                }

                // Normal index access
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
        let right = &expr_str[dot_pos + 1..];
        // Check if right side looks like an attribute identifier
        if right
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
            && right.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
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
        message: format!("Cannot parse expression: {expr_str}"),
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_assignment() {
        let result = parse("x: int = 10").unwrap();
        assert_eq!(result.statements.len(), 1);
        if let Stmt::Assign {
            target,
            type_hint,
            value,
        } = &result.statements[0]
        {
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
        if let Stmt::FuncDef {
            name,
            params,
            return_type,
            body,
        } = &result.statements[0]
        {
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
        if let Stmt::If {
            then_body,
            else_body,
            ..
        } = &result.statements[0]
        {
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
