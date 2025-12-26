//! Parser utility functions - bracket matching, string parsing, etc.

/// Find first occurrence of a char respecting bracket balance and strings (Left to Right)
pub fn find_char_balanced(s: &str, target: char) -> Option<usize> {
    let mut depth_paren = 0;
    let mut depth_bracket = 0;
    let mut depth_brace = 0;
    let mut in_string = false;
    let mut string_quote = '\0';
    let mut escaped = false;
    
    for (i, c) in s.char_indices() {
        if in_string {
             if escaped {
                 escaped = false;
             } else if c == '\\' {
                 escaped = true;
             } else if c == string_quote {
                 in_string = false;
             }
             continue;
        }
        
        match c {
            '"' | '\'' => {
                in_string = true;
                string_quote = c;
            }
            '(' => depth_paren += 1,
            ')' => if depth_paren > 0 { depth_paren -= 1 },
            '[' => depth_bracket += 1,
            ']' => if depth_bracket > 0 { depth_bracket -= 1 },
            '{' => depth_brace += 1,
            '}' => if depth_brace > 0 { depth_brace -= 1 },
            _ if c == target && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 => {
                 return Some(i);
            }
            _ => {}
        }
    }
    None
}

/// Find matching opening bracket starting from a position (moving Left)
pub fn find_matching_bracket_rtl(s: &str, end_pos: usize, close_char: char, open_char: char) -> Option<usize> {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut string_quote = '\0';
    let mut escaped = false;
    
    for (i, c) in s.char_indices() {
        if i > end_pos {
            break; 
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == string_quote {
                in_string = false;
            }
            continue;
        }
        
        if c == '"' || c == '\'' {
            in_string = true;
            string_quote = c;
            escaped = false;
            continue;
        }
        
        if c == open_char {
            stack.push(i);
        } else if c == close_char {
            if let Some(open_pos) = stack.pop() {
                if i == end_pos {
                    return Some(open_pos);
                }
            }
        }
    }
    None
}

/// Find matching bracket starting from a position (Left to Right)
pub fn find_matching_bracket(s: &str, start: usize, open: char, close: char) -> Option<usize> {
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
pub fn find_char_balanced_rtl(s: &str, target: char) -> Option<usize> {
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
pub fn split_by_comma_balanced(s: &str) -> Vec<String> {
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
pub fn find_operator_balanced(s: &str, op: &str) -> Option<usize> {
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
pub fn find_operator_balanced_rtl(s: &str, op: &str) -> Option<usize> {
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
pub fn find_keyword_balanced(s: &str, keyword: &str) -> Option<usize> {
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
    fn test_find_char_balanced() {
        assert_eq!(find_char_balanced("a = b", '='), Some(2));
        assert_eq!(find_char_balanced("a == b", '='), Some(2));
        assert_eq!(find_char_balanced("func('=')", '='), None);
    }

    #[test]
    fn test_split_by_comma_balanced() {
        let parts = split_by_comma_balanced("a, b, c");
        assert_eq!(parts, vec!["a", "b", "c"]);
        
        let parts = split_by_comma_balanced("func(a, b), c");
        assert_eq!(parts, vec!["func(a, b)", "c"]);
    }
}
