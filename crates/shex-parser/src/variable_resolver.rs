//! Variable resolution infrastructure for Shex parser
//!
//! Provides the foundation for parameter expansion, variable scoping,
//! and context-aware string resolution needed for POSIX shell behavior.

use std::collections::HashMap;

/// Variable resolution context for parameter expansion
///
/// This will be extended to support different expansion modes,
/// error handling, and nested contexts as we implement more POSIX features
#[derive(Debug, Clone)]
pub struct VariableContext {
    /// Current variable bindings
    variables: HashMap<String, String>,
    /// Parent context for nested scopes (future use)
    parent: Option<Box<VariableContext>>,
}

impl VariableContext {
    /// Create a new empty variable context
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    /// Create a new context with a parent for nested scoping
    #[must_use]
    pub fn with_parent(parent: VariableContext) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Set a variable in the current context
    pub fn set(&mut self, name: String, value: String) {
        self.variables.insert(name, value);
    }

    /// Get a variable value, checking parent contexts if not found locally
    pub fn get(&self, name: &str) -> Option<&String> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(name)))
    }

    /// Check if a variable exists in any accessible context
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name)
            || self
                .parent
                .as_ref()
                .map_or(false, |parent| parent.contains(name))
    }

    /// Get all variable names from all accessible contexts
    pub fn all_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.variables.keys().cloned().collect();
        if let Some(parent) = &self.parent {
            let mut parent_names = parent.all_names();
            parent_names.retain(|name| !self.variables.contains_key(name));
            names.extend(parent_names);
        }
        names.sort();
        names
    }

    /// Import variables from another context (shallow copy)
    pub fn import_from(&mut self, other: &VariableContext) {
        for (name, value) in &other.variables {
            self.variables.insert(name.clone(), value.clone());
        }
    }

    /// Get a copy of all variables in the current context only
    pub fn current_variables(&self) -> HashMap<String, String> {
        self.variables.clone()
    }
}

impl Default for VariableContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameter expansion mode for future POSIX compliance
///
/// This enum will be used when we implement full parameter expansion
/// to handle different expansion behaviors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionMode {
    /// Normal expansion: $var or ${var}
    Normal,
    /// Default value: ${var:-default}
    DefaultValue,
    /// Assign default: ${var:=default}
    AssignDefault,
    /// Error if unset: ${var:?message}
    ErrorIfUnset,
    /// Alternative value: ${var:+value}
    AlternativeValue,
}

/// Parameter expansion request
///
/// This struct will be used when we implement parameter expansion
/// to represent expansion requests and their context
#[derive(Debug, Clone)]
pub struct ExpansionRequest {
    /// Variable name to expand
    pub variable_name: String,
    /// Expansion mode
    pub mode: ExpansionMode,
    /// Optional parameter for expansion modes that need it
    pub parameter: Option<String>,
    /// Whether to check for unset (: prefix in expansion)
    pub check_unset: bool,
}

impl ExpansionRequest {
    /// Create a simple variable expansion request
    #[must_use]
    pub fn simple(variable_name: String) -> Self {
        Self {
            variable_name,
            mode: ExpansionMode::Normal,
            parameter: None,
            check_unset: false,
        }
    }

    /// Create an expansion request with default value
    #[must_use]
    pub fn with_default(variable_name: String, default_value: String) -> Self {
        Self {
            variable_name,
            mode: ExpansionMode::DefaultValue,
            parameter: Some(default_value),
            check_unset: false,
        }
    }
}

/// Variable resolution result
///
/// Used to communicate the result of variable resolution and
/// any side effects (like assignments) that occurred
#[derive(Debug, Clone)]
pub enum ResolutionResult {
    /// Variable resolved successfully
    Resolved(String),
    /// Variable is unset (different from empty)
    Unset,
    /// Resolution resulted in an error
    Error(String),
}

/// Resolve a variable expansion request
///
/// This function will be expanded to handle all POSIX parameter expansion
/// modes as we implement them
pub fn resolve_expansion(
    context: &mut VariableContext,
    request: &ExpansionRequest,
) -> ResolutionResult {
    match request.mode {
        ExpansionMode::Normal => match context.get(&request.variable_name) {
            Some(value) => ResolutionResult::Resolved(value.clone()),
            None => ResolutionResult::Unset,
        },
        ExpansionMode::DefaultValue => match context.get(&request.variable_name) {
            Some(value) if !value.is_empty() || !request.check_unset => {
                ResolutionResult::Resolved(value.clone())
            }
            _ => match &request.parameter {
                Some(default) => ResolutionResult::Resolved(default.clone()),
                None => ResolutionResult::Error(
                    "Default value expansion requires parameter".to_string(),
                ),
            },
        },
        ExpansionMode::AssignDefault => match context.get(&request.variable_name) {
            Some(value) if !value.is_empty() || !request.check_unset => {
                ResolutionResult::Resolved(value.clone())
            }
            _ => match &request.parameter {
                Some(default) => {
                    context.set(request.variable_name.clone(), default.clone());
                    ResolutionResult::Resolved(default.clone())
                }
                None => ResolutionResult::Error(
                    "Assign default expansion requires parameter".to_string(),
                ),
            },
        },
        ExpansionMode::ErrorIfUnset => match context.get(&request.variable_name) {
            Some(value) if !value.is_empty() || !request.check_unset => {
                ResolutionResult::Resolved(value.clone())
            }
            _ => {
                let message = request.parameter.as_ref().map_or_else(
                    || format!("{}: parameter null or not set", request.variable_name),
                    |msg| msg.clone(),
                );
                ResolutionResult::Error(message)
            }
        },
        ExpansionMode::AlternativeValue => match context.get(&request.variable_name) {
            Some(value) if !value.is_empty() || !request.check_unset => match &request.parameter {
                Some(alternative) => ResolutionResult::Resolved(alternative.clone()),
                None => ResolutionResult::Resolved(String::new()),
            },
            _ => ResolutionResult::Resolved(String::new()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_variable_context() {
        let mut context = VariableContext::new();

        assert!(context.get("var").is_none());
        assert!(!context.contains("var"));

        context.set("var".to_string(), "value".to_string());

        assert_eq!(context.get("var"), Some(&"value".to_string()));
        assert!(context.contains("var"));
    }

    #[test]
    fn test_nested_context() {
        let mut parent = VariableContext::new();
        parent.set("parent_var".to_string(), "parent_value".to_string());

        let mut child = VariableContext::with_parent(parent);
        child.set("child_var".to_string(), "child_value".to_string());

        assert_eq!(child.get("child_var"), Some(&"child_value".to_string()));
        assert_eq!(child.get("parent_var"), Some(&"parent_value".to_string()));
        assert!(child.contains("parent_var"));

        // Child variables shadow parent
        child.set("parent_var".to_string(), "overridden".to_string());
        assert_eq!(child.get("parent_var"), Some(&"overridden".to_string()));
    }

    #[test]
    fn test_all_names() {
        let mut parent = VariableContext::new();
        parent.set("a".to_string(), "1".to_string());
        parent.set("b".to_string(), "2".to_string());

        let mut child = VariableContext::with_parent(parent);
        child.set("c".to_string(), "3".to_string());
        child.set("a".to_string(), "overridden".to_string()); // Should not duplicate

        let names = child.all_names();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_import_from() {
        let mut source = VariableContext::new();
        source.set("var1".to_string(), "value1".to_string());
        source.set("var2".to_string(), "value2".to_string());

        let mut target = VariableContext::new();
        target.import_from(&source);

        assert_eq!(target.get("var1"), Some(&"value1".to_string()));
        assert_eq!(target.get("var2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_expansion_request_creation() {
        let simple = ExpansionRequest::simple("var".to_string());
        assert_eq!(simple.variable_name, "var");
        assert_eq!(simple.mode, ExpansionMode::Normal);
        assert!(simple.parameter.is_none());

        let with_default = ExpansionRequest::with_default("var".to_string(), "default".to_string());
        assert_eq!(with_default.mode, ExpansionMode::DefaultValue);
        assert_eq!(with_default.parameter, Some("default".to_string()));
    }

    #[test]
    fn test_normal_expansion() {
        let mut context = VariableContext::new();
        context.set("var".to_string(), "value".to_string());

        let request = ExpansionRequest::simple("var".to_string());
        let result = resolve_expansion(&mut context, &request);

        match result {
            ResolutionResult::Resolved(value) => assert_eq!(value, "value"),
            _ => panic!("Expected resolved result"),
        }

        let unset_request = ExpansionRequest::simple("unset_var".to_string());
        let unset_result = resolve_expansion(&mut context, &unset_request);

        match unset_result {
            ResolutionResult::Unset => {}
            _ => panic!("Expected unset result"),
        }
    }

    #[test]
    fn test_default_value_expansion() {
        let mut context = VariableContext::new();

        let request =
            ExpansionRequest::with_default("unset_var".to_string(), "default_value".to_string());
        let result = resolve_expansion(&mut context, &request);

        match result {
            ResolutionResult::Resolved(value) => assert_eq!(value, "default_value"),
            _ => panic!("Expected resolved result with default value"),
        }

        // Test with existing variable
        context.set("var".to_string(), "existing".to_string());
        let existing_request =
            ExpansionRequest::with_default("var".to_string(), "default".to_string());
        let existing_result = resolve_expansion(&mut context, &existing_request);

        match existing_result {
            ResolutionResult::Resolved(value) => assert_eq!(value, "existing"),
            _ => panic!("Expected existing value, not default"),
        }
    }

    #[test]
    fn test_assign_default_expansion() {
        let mut context = VariableContext::new();

        let mut request =
            ExpansionRequest::with_default("unset_var".to_string(), "default_value".to_string());
        request.mode = ExpansionMode::AssignDefault;

        let result = resolve_expansion(&mut context, &request);

        match result {
            ResolutionResult::Resolved(value) => assert_eq!(value, "default_value"),
            _ => panic!("Expected resolved result"),
        }

        // Verify variable was set
        assert_eq!(context.get("unset_var"), Some(&"default_value".to_string()));
    }
}
