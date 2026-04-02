//! Lint rule registry for the Arukellt compiler.
//!
//! Provides a registry of lint rules with configurable severity levels.
//! Each lint rule has a unique ID, category, description, and default severity.
//! Users can override severity via `allow`, `warn`, or `deny`.

use crate::codes::DiagnosticCode;

/// Severity level for a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintLevel {
    /// The lint is suppressed.
    Allow,
    /// The lint emits a warning.
    Warn,
    /// The lint is treated as a hard error.
    Deny,
}

/// Category of a lint rule, used for grouping and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintCategory {
    /// Type safety and semantic correctness.
    Correctness,
    /// Code style and formatting conventions.
    Style,
    /// Performance-related suggestions.
    Performance,
    /// Hints for migration between language versions.
    Migration,
    /// Potential issues in target-specific code generation.
    Target,
    /// Backend validation issues.
    Backend,
}

impl LintCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Correctness => "correctness",
            Self::Style => "style",
            Self::Performance => "performance",
            Self::Migration => "migration",
            Self::Target => "target",
            Self::Backend => "backend",
        }
    }
}

/// A single lint rule definition.
#[derive(Debug, Clone)]
pub struct LintRule {
    /// The diagnostic code this rule maps to.
    pub code: DiagnosticCode,
    /// Human-readable identifier (e.g., "W0001").
    pub id: &'static str,
    /// Short description of what this lint checks.
    pub description: &'static str,
    /// Category for grouping.
    pub category: LintCategory,
    /// Default severity level.
    pub default_level: LintLevel,
    /// Whether an automatic fix is available.
    pub has_fix: bool,
}

/// Registry of all known lint rules.
pub struct LintRegistry {
    rules: Vec<LintRule>,
}

impl LintRegistry {
    /// Create a new registry populated with all built-in lint rules.
    pub fn new() -> Self {
        let rules = vec![
            LintRule {
                code: DiagnosticCode::W0001,
                id: "W0001",
                description: "possible unintended sharing of reference type",
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
                has_fix: false,
            },
            LintRule {
                code: DiagnosticCode::W0002,
                id: "W0002",
                description: "deprecated target alias",
                category: LintCategory::Target,
                default_level: LintLevel::Warn,
                has_fix: false,
            },
            LintRule {
                code: DiagnosticCode::W0003,
                id: "W0003",
                description: "ambiguous import: local and std modules share the same name",
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
                has_fix: false,
            },
            LintRule {
                code: DiagnosticCode::W0004,
                id: "W0004",
                description: "generated Wasm module failed validation",
                category: LintCategory::Backend,
                default_level: LintLevel::Deny,
                has_fix: false,
            },
            LintRule {
                code: DiagnosticCode::W0005,
                id: "W0005",
                description: "function has non-exportable parameter type, skipped from component exports",
                category: LintCategory::Backend,
                default_level: LintLevel::Warn,
                has_fix: false,
            },
            LintRule {
                code: DiagnosticCode::W0006,
                id: "W0006",
                description: "unused import",
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
                has_fix: true,
            },
            LintRule {
                code: DiagnosticCode::W0007,
                id: "W0007",
                description: "unused binding",
                category: LintCategory::Correctness,
                default_level: LintLevel::Warn,
                has_fix: true,
            },
        ];
        Self { rules }
    }

    /// Return all registered lint rules.
    pub fn rules(&self) -> &[LintRule] {
        &self.rules
    }

    /// Look up a lint rule by its string ID.
    pub fn find_by_id(&self, id: &str) -> Option<&LintRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Look up a lint rule by its diagnostic code.
    pub fn find_by_code(&self, code: DiagnosticCode) -> Option<&LintRule> {
        self.rules.iter().find(|r| r.code == code)
    }

    /// Return all rules in a given category.
    pub fn rules_in_category(&self, category: LintCategory) -> Vec<&LintRule> {
        self.rules
            .iter()
            .filter(|r| r.category == category)
            .collect()
    }

    /// Return the number of registered rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl Default for LintRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_builtin_rules() {
        let reg = LintRegistry::new();
        assert_eq!(reg.len(), 7);
    }

    #[test]
    fn find_by_id() {
        let reg = LintRegistry::new();
        let rule = reg.find_by_id("W0001").unwrap();
        assert_eq!(rule.code, DiagnosticCode::W0001);
        assert_eq!(rule.category, LintCategory::Correctness);
    }

    #[test]
    fn find_by_code() {
        let reg = LintRegistry::new();
        let rule = reg.find_by_code(DiagnosticCode::W0005).unwrap();
        assert_eq!(rule.id, "W0005");
    }

    #[test]
    fn category_filter() {
        let reg = LintRegistry::new();
        let backend = reg.rules_in_category(LintCategory::Backend);
        assert_eq!(backend.len(), 2);
    }

    #[test]
    fn w0004_defaults_to_deny() {
        let reg = LintRegistry::new();
        let rule = reg.find_by_id("W0004").unwrap();
        assert_eq!(rule.default_level, LintLevel::Deny);
    }

    #[test]
    fn all_rules_have_descriptions() {
        let reg = LintRegistry::new();
        for rule in reg.rules() {
            assert!(
                !rule.description.is_empty(),
                "rule {} has empty description",
                rule.id
            );
        }
    }
}
