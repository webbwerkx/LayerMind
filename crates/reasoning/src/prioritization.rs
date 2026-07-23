//! Recommendation prioritization — deterministic action ordering.
//!
//! After the AI returns actions, this module re-orders them by
//! deterministic priority rules so the operator always sees the
//! highest-impact action first.
//!
//! Priority scoring:
//!   severity_weight * 0.30 + safety_weight * 0.20 + health_impact * 0.20
//!   + historical_relevance * 0.15 + actionability * 0.15
//!
//! Higher score = higher priority. After sorting, priority numbers
//! are reassigned (1 = highest).

use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::Action;

/// Re-orders recommendations by deterministic priority rules.
#[derive(Debug, Default)]
pub struct Prioritizer;

impl Prioritizer {
    pub fn new() -> Self {
        Self
    }

    /// Sort actions by priority and reassign priority numbers.
    /// Returns the ordered actions.
    pub fn prioritize(&self, actions: &mut Vec<Action>, context: &PrinterContext) {
        if actions.len() <= 1 {
            return;
        }

        // Score each action.
        let mut scored: Vec<(usize, f64)> = actions
            .iter()
            .enumerate()
            .map(|(idx, action)| (idx, self.score(action, context)))
            .collect();

        // Sort by score descending, stable by original position.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Reorder.
        let reordered: Vec<Action> = scored
            .iter()
            .map(|(idx, _)| actions[*idx].clone())
            .collect();

        *actions = reordered;

        // Reassign priority numbers.
        for (i, action) in actions.iter_mut().enumerate() {
            action.priority = (i + 1) as u8;
        }
    }

    fn score(&self, action: &Action, context: &PrinterContext) -> f64 {
        // ── Safety weight ────────
        // Non-automatic actions get higher priority — they need operator attention.
        let safety_weight = if action.is_safe_automatic { 0.5 } else { 1.0 };

        // ── Health impact ────────
        // If the action's description matches an active warning or known issue,
        // it likely has direct health impact.
        let health_match = context
            .current_state
            .pending_warnings
            .iter()
            .any(|w| action_description_matches(action, w))
            || context
                .known_issues
                .iter()
                .any(|i| !i.resolved && action_description_matches(action, &i.description));
        let health_impact = if health_match { 1.0 } else { 0.3 };

        // ── Historical relevance ─
        // If a similar pattern has been seen before, this action is more urgent.
        let historical_relevance = if context
            .historical_patterns
            .iter()
            .any(|p| action_description_matches(action, &p.description))
        {
            0.8
        } else {
            0.5
        };

        // ── Actionability ────────
        // Having a suggested command means the action is directly executable.
        let actionability = if action.suggested_command.is_some() {
            1.0
        } else {
            0.6
        };

        safety_weight * 0.30
            + health_impact * 0.20
            + historical_relevance * 0.15
            + actionability * 0.15
            + 0.20
        // base weight so all actions have non-zero score
    }
}

/// Loose keyword match between an action description and a context string.
fn action_description_matches(action: &Action, context_text: &str) -> bool {
    let desc_lower = action.description.to_lowercase();
    let ctx_lower = context_text.to_lowercase();

    // Split into words ≥ 4 chars and check for matches.
    let keywords: Vec<&str> = desc_lower
        .split_whitespace()
        .filter(|w| w.len() >= 4)
        .collect();

    if keywords.is_empty() {
        return false;
    }

    keywords.iter().any(|kw| ctx_lower.contains(*kw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_shared::context::{
        CurrentState, HealthSummary, HistoricalPattern, IssueSummary, PrintHistorySummary,
        PrinterSummary,
    };

    fn test_context() -> PrinterContext {
        PrinterContext {
            printer_id: "p1".into(),
            generated_at: chrono::Utc::now(),
            summary: PrinterSummary::default(),
            print_history: PrintHistorySummary::default(),
            health: HealthSummary::default(),
            current_state: CurrentState {
                pending_warnings: vec!["temperature instability detected".into()],
                ..Default::default()
            },
            known_issues: vec![IssueSummary {
                category: "thermal".into(),
                description: "PID oscillation on extruder".into(),
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrence_count: 3,
                resolved: false,
                importance: 0.7,
            }],
            historical_patterns: vec![HistoricalPattern {
                pattern_type: "thermal".into(),
                description: "temperature instability recurring".into(),
                occurrence_count: 3,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                typical_severity: "warning".into(),
                resolved_count: 0,
            }],
            machine: None,
            recent_evidence: Vec::new(),
        }
    }

    #[test]
    fn prioritizes_health_impact_actions() {
        let mut actions = vec![
            Action {
                priority: 1,
                description: "Clean print bed".into(),
                suggested_command: None,
                expected_outcome: "Better adhesion".into(),
                is_safe_automatic: true,
            },
            Action {
                priority: 2,
                description: "Run PID calibration to fix temperature oscillation".into(),
                suggested_command: Some("PID_CALIBRATE HEATER=extruder TARGET=210".into()),
                expected_outcome: "Stable temperature".into(),
                is_safe_automatic: false,
            },
        ];

        let ctx = test_context();
        let prioritizer = Prioritizer::new();
        prioritizer.prioritize(&mut actions, &ctx);

        // PID calibration should be first (matches active warning).
        assert!(actions[0].description.contains("PID"));
        assert_eq!(actions[0].priority, 1);
        assert_eq!(actions[1].priority, 2);
    }

    #[test]
    fn single_action_unchanged() {
        let mut actions = vec![Action {
            priority: 1,
            description: "Clean nozzle".into(),
            suggested_command: None,
            expected_outcome: "Clean prints".into(),
            is_safe_automatic: true,
        }];

        let ctx = PrinterContext::new("p1".into());
        let prioritizer = Prioritizer::new();
        prioritizer.prioritize(&mut actions, &ctx);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].priority, 1);
    }

    #[test]
    fn empty_actions_unchanged() {
        let mut actions: Vec<Action> = Vec::new();
        let ctx = PrinterContext::new("p1".into());
        let prioritizer = Prioritizer::new();
        prioritizer.prioritize(&mut actions, &ctx);
        assert!(actions.is_empty());
    }
}
