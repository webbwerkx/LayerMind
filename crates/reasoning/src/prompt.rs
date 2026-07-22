//! Prompt builder — converts PrinterContext into structured AI prompts.
//!
//! Produces a system prompt (instructions for the AI role) and a user
//! prompt (the specific printer context to reason about).

use layermind_shared::context::PrinterContext;

/// Builds prompts from a PrinterContext.
///
/// The system prompt defines the AI's role as a 3D printer diagnostic
/// expert. The user prompt is a structured JSON representation of the
/// printer's current state, optimized for AI consumption.
#[derive(Debug, Default)]
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build the system prompt — defines the AI's role and output format.
    pub fn system_prompt(&self) -> String {
        include_str!("../prompts/system.md").to_string()
    }

    /// Build the user prompt — the printer context to reason about.
    pub fn user_prompt(&self, context: &PrinterContext) -> String {
        serde_json::to_string_pretty(context).unwrap_or_else(|_| {
            format!(
                "Printer context for {} (serialization failed)",
                context.printer_id
            )
        })
    }

    /// Build the complete prompt pair.
    pub fn build(&self, context: &PrinterContext) -> PromptPair {
        PromptPair {
            system: self.system_prompt(),
            user: self.user_prompt(context),
        }
    }
}

/// A complete prompt pair ready to send to an AI provider.
#[derive(Debug, Clone)]
pub struct PromptPair {
    pub system: String,
    pub user: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::context::{
        CurrentState, HealthSummary, PrintHistorySummary, PrinterSummary,
    };

    fn basic_context() -> PrinterContext {
        PrinterContext {
            printer_id: "test-printer".into(),
            generated_at: Utc::now(),
            summary: PrinterSummary {
                name: "Test Printer".into(),
                model: Some("Ender 3 V2".into()),
                firmware: Some("Marlin 2.1".into()),
                ..Default::default()
            },
            print_history: PrintHistorySummary {
                total_prints: 10,
                successful_prints: 8,
                failed_prints: 2,
                success_rate: Some(0.8),
                ..Default::default()
            },
            health: HealthSummary::default(),
            current_state: CurrentState::default(),
            known_issues: Vec::new(),
            historical_patterns: Vec::new(),
            recent_evidence: Vec::new(),
        }
    }

    #[test]
    fn system_prompt_is_non_empty() {
        let builder = PromptBuilder::new();
        let prompt = builder.system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("3D printer"));
    }

    #[test]
    fn user_prompt_includes_printer_info() {
        let builder = PromptBuilder::new();
        let ctx = basic_context();
        let prompt = builder.user_prompt(&ctx);
        assert!(prompt.contains("test-printer"));
        assert!(prompt.contains("Ender 3 V2"));
    }

    #[test]
    fn prompt_pair_contains_both_prompts() {
        let builder = PromptBuilder::new();
        let ctx = basic_context();
        let pair = builder.build(&ctx);
        assert!(!pair.system.is_empty());
        assert!(!pair.user.is_empty());
    }
}
