//! Contradiction detection — finds conflicting evidence and state
//! in a PrinterContext before AI diagnosis.
//!
//! Contradictions are surfaced in diagnostic metadata so the operator
//! can investigate conflicting sensor data or inconsistent printer
//! state. They are never silently ignored.
//!
//! Detection rules (deterministic, no AI):
//!   1. Issue marked resolved but still present in active observations.
//!   2. Temperature stability score contradicts known temperature issues.
//!   3. High success rate conflicts with frequent failure patterns.
//!   4. Print state contradicts active warnings.

use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::{Contradiction, ContradictionSeverity};

/// Detects contradictions within a PrinterContext.
#[derive(Debug, Default)]
pub struct ContradictionDetector;

impl ContradictionDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect all contradictions in the given context.
    pub fn detect(&self, context: &PrinterContext) -> Vec<Contradiction> {
        let mut contradictions = Vec::new();

        // Rule 1: Resolved issues still in active observations.
        for issue in &context.known_issues {
            if issue.resolved {
                let still_active = context
                    .current_state
                    .active_observations
                    .iter()
                    .any(|obs| obs.category == issue.category && !obs.message.is_empty());

                if still_active {
                    contradictions.push(Contradiction {
                        description: format!(
                            "Issue '{}' is marked resolved but still has active observations",
                            issue.description
                        ),
                        item_a: format!("Resolved issue: {}", issue.description),
                        item_b: "Active observation matching resolved issue".into(),
                        severity: ContradictionSeverity::Significant,
                    });
                }
            }
        }

        // Rule 2: Temperature stability contradicts known temperature issues.
        if let (Some(stability), true) = (
            context.health.temperature_stability,
            !context.known_issues.is_empty(),
        ) {
            let has_thermal_issue = context
                .known_issues
                .iter()
                .any(|i| i.category.contains("temperature") && !i.resolved);

            if stability > 0.9 && has_thermal_issue {
                contradictions.push(Contradiction {
                    description:
                        "Temperature stability score is high (≥0.9) but unresolved temperature issues exist"
                            .into(),
                    item_a: format!("Temperature stability: {:.2}", stability),
                    item_b: "Unresolved temperature issue present".into(),
                    severity: ContradictionSeverity::Minor,
                });
            }
        }

        // Rule 3: High success rate conflicts with frequent failures.
        if let Some(success_rate) = context.print_history.success_rate {
            let failures = context.print_history.recent_failures.len();
            if success_rate > 0.9 && failures > 2 {
                contradictions.push(Contradiction {
                    description: format!(
                        "High success rate ({:.0}%) but {} recent failures detected",
                        success_rate * 100.0,
                        failures
                    ),
                    item_a: format!("Success rate: {:.0}%", success_rate * 100.0),
                    item_b: format!("{} recent failures", failures),
                    severity: ContradictionSeverity::Minor,
                });
            }
        }

        // Rule 4: Print-state vs warnings mismatch.
        if !context.current_state.is_printing && !context.current_state.pending_warnings.is_empty()
        {
            let has_active_issue = context.known_issues.iter().any(|i| !i.resolved);

            if has_active_issue {
                contradictions.push(Contradiction {
                    description:
                        "Printer is idle but has active unresolved issues and pending warnings"
                            .into(),
                    item_a: "Printer state: idle".into(),
                    item_b: format!(
                        "{} active issues, {} pending warnings",
                        context.known_issues.iter().filter(|i| !i.resolved).count(),
                        context.current_state.pending_warnings.len()
                    ),
                    severity: ContradictionSeverity::Significant,
                });
            }
        }

        // Rule 5: Conflicting observation categories.
        if !context.current_state.active_observations.is_empty() {
            let categories: std::collections::HashSet<&str> = context
                .current_state
                .active_observations
                .iter()
                .map(|o| o.category.as_str())
                .collect();

            // Check for opposing categories.
            let has_temp_issue = categories.contains("temperature_instability");
            let has_cooling_issue = categories.contains("cooling_issue");

            if has_temp_issue && has_cooling_issue {
                contradictions.push(Contradiction {
                    description:
                        "Both temperature instability and cooling issues detected — these may have opposing root causes"
                            .into(),
                    item_a: "Temperature instability detected".into(),
                    item_b: "Cooling issue detected".into(),
                    severity: ContradictionSeverity::Minor,
                });
            }
        }

        contradictions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::context::{
        CurrentState, HealthSummary, IssueSummary, ObservationSummary, PrintHistorySummary,
        PrinterSummary, RecentFailure,
    };

    fn base_context() -> PrinterContext {
        PrinterContext::new("p1".into())
    }

    #[test]
    fn no_contradictions_in_clean_context() {
        let ctx = base_context();
        let detector = ContradictionDetector::new();
        let contradictions = detector.detect(&ctx);
        assert!(contradictions.is_empty());
    }

    #[test]
    fn detects_resolved_but_active_issue() {
        let mut ctx = base_context();
        ctx.known_issues.push(IssueSummary {
            category: "temperature_instability".into(),
            description: "PID oscillation".into(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            occurrence_count: 1,
            resolved: true,
            importance: 0.5,
        });
        ctx.current_state
            .active_observations
            .push(ObservationSummary {
                category: "temperature_instability".into(),
                severity: "warning".into(),
                message: "Still oscillating".into(),
                importance: 0.5,
                confidence: 0.8,
                quality: layermind_shared::context::EvidenceQuality::Observed,
                timestamp: Utc::now(),
            });

        let detector = ContradictionDetector::new();
        let contradictions = detector.detect(&ctx);
        assert_eq!(contradictions.len(), 1);
        assert!(contradictions[0].description.contains("resolved"));
    }

    #[test]
    fn idle_with_active_issues() {
        let mut ctx = base_context();
        ctx.known_issues.push(IssueSummary {
            category: "mechanical".into(),
            description: "Belt tension".into(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            occurrence_count: 1,
            resolved: false,
            importance: 0.6,
        });
        ctx.current_state
            .pending_warnings
            .push("Belt may be loose".into());

        let detector = ContradictionDetector::new();
        let contradictions = detector.detect(&ctx);
        assert!(
            contradictions
                .iter()
                .any(|c| c.description.contains("idle"))
        );
    }
}
