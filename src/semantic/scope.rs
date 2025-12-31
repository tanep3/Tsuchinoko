//! Scope management

use super::Type;
use std::collections::HashMap;

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
    /// Type narrowing: variables whose type is narrowed in this scope
    /// (e.g., after `if x is None:` check, x's type becomes T instead of Option<T>)
    narrowed_types: HashMap<String, Type>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            narrowed_types: HashMap::new(),
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

    pub fn narrow_type(&mut self, name: &str, ty: Type) {
        self.narrowed_types.insert(name.to_string(), ty);
    }

    pub fn get_narrowed_type(&self, name: &str) -> Option<&Type> {
        self.narrowed_types.get(name)
    }
}

/// Stack of scopes for nested contexts
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeStack {
    pub fn depth(&self) -> usize {
        self.scopes.len().saturating_sub(1)
    }

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

    /// Narrow the type of a variable in the current scope
    pub fn narrow_type(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.narrow_type(name, ty);
        }
    }

    /// Get the effective type of a variable (considering narrowing)
    pub fn get_effective_type(&self, name: &str) -> Option<Type> {
        // First check if the type is narrowed in any scope (most recent first)
        for scope in self.scopes.iter().rev() {
            if let Some(narrowed) = scope.get_narrowed_type(name) {
                return Some(narrowed.clone());
            }
        }
        // Fall back to original type
        self.lookup(name).map(|info| info.ty.clone())
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
