//! Scope management

use std::collections::HashMap;
use super::Type;

/// Variable information
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

/// Scope for variable tracking
#[derive(Debug, Clone)]
pub struct Scope {
    variables: HashMap<String, VarInfo>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: &str, ty: Type, mutable: bool) {
        self.variables.insert(
            name.to_string(),
            VarInfo {
                name: name.to_string(),
                ty,
                mutable,
            },
        );
    }

    pub fn lookup(&self, name: &str) -> Option<&VarInfo> {
        self.variables.get(name)
    }
}

/// Stack of scopes for nested contexts
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()], // Global scope
        }
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn pop(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, name: &str, ty: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, ty, mutable);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.lookup(name) {
                return Some(info);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_define_and_lookup() {
        let mut scope = Scope::new();
        scope.define("x", Type::Int, false);
        
        let info = scope.lookup("x").unwrap();
        assert_eq!(info.name, "x");
        assert_eq!(info.ty, Type::Int);
    }

    #[test]
    fn test_scope_stack_nested() {
        let mut stack = ScopeStack::new();
        stack.define("global_var", Type::Int, false);
        
        stack.push();
        stack.define("local_var", Type::String, true);
        
        assert!(stack.lookup("global_var").is_some());
        assert!(stack.lookup("local_var").is_some());
        
        stack.pop();
        assert!(stack.lookup("global_var").is_some());
        assert!(stack.lookup("local_var").is_none());
    }
}
