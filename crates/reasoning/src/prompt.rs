//! Prompt builder — converts PrinterContext into optimized AI prompts.
//!
//! Phase 2.3 improvements:
//! - Evidence ranked by recency/confidence/relevance (via EvidenceRanker)
//! - Historical trends included (recurring/new/improving/worsening)
//! - Contradictions surfaced in the prompt
//! - Concise, structured format to minimize token usage
//! - Stronger instructions for multi-issue diagnosis and explainability

use crate::evidence::{EvidenceRanker, RankedContext};
use crate::strategy::DiagnosticStrategy;
use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::{Contradiction, Trend};

/// Builds prompts from a PrinterContext with ranked evidence and
/// historical comparison.
pub struct PromptBuilder {
    ranker: EvidenceRanker,
    strategy: DiagnosticStrategy,
}

impl PromptBuilder {
    pub fn new(strategy: DiagnosticStrategy) -> Self {
        Self {
            ranker: EvidenceRanker::new(strategy.clone()),
            strategy,
        }
    }

    /// Build the system prompt — defines the AI's role and output format.
    pub fn system_prompt(&self) -> String {
        include_str!("../prompts/system.md").to_string()
    }

    /// Build the user prompt — ranked, historical, contradiction-aware.
    pub fn user_prompt(
        &self,
        context: &PrinterContext,
        contradictions: &[Contradiction],
    ) -> String {
        let ranked = self.ranker.rank(context);
        self.build_structured_prompt(context, &ranked, contradictions)
    }

    /// Build the complete prompt pair.
    pub fn build(&self, context: &PrinterContext, contradictions: &[Contradiction]) -> PromptPair {
        PromptPair {
            system: self.system_prompt(),
            user: self.user_prompt(context, contradictions),
        }
    }

    fn build_structured_prompt(
        &self,
        context: &PrinterContext,
        ranked: &RankedContext,
        contradictions: &[Contradiction],
    ) -> String {
        let mut sections = Vec::new();

        // ── 1. Printer identity ────────────────────
        sections.push(format!("## Printer: {}", context.summary.name));
        if let Some(ref model) = context.summary.model {
            sections.push(format!("Model: {}", model));
        }
        if let Some(ref fw) = context.summary.firmware {
            sections.push(format!("Firmware: {}", fw));
        }
        sections.push(format!(
            "Status: {}",
            if context.current_state.is_printing {
                "printing"
            } else {
                "idle"
            }
        ));

        // ── 2. Health snapshot ─────────────────────
        let health = &context.health;
        let mut health_lines = vec!["## Health".to_string()];
        if let Some(stab) = health.temperature_stability {
            health_lines.push(format!("- Temperature stability: {:.2}", stab));
        }
        if let Some(sr) = health.success_rate {
            health_lines.push(format!("- Success rate: {:.0}%", sr * 100.0));
        }
        if let Some(rel) = health.reliability_score {
            health_lines.push(format!("- Reliability: {:.2}", rel));
        }
        health_lines.push(format!("- Recent errors: {}", health.recent_error_count));
        health_lines.push(format!(
            "- Recent warnings: {}",
            health.recent_warning_count
        ));
        sections.push(health_lines.join("\n"));

        // ── 3. Print history ──────────────────────
        let hist = &context.print_history;
        let mut hist_lines = vec!["## Print History".to_string()];
        hist_lines.push(format!(
            "- {} total / {} succeeded / {} failed",
            hist.total_prints, hist.successful_prints, hist.failed_prints
        ));
        if let Some(avg) = hist.avg_duration_secs {
            hist_lines.push(format!("- Average duration: {:.0}s", avg));
        }
        if !hist.recent_failures.is_empty() {
            hist_lines.push(format!("- {} recent failures", hist.recent_failures.len()));
            // Show last N failure reasons based on strategy.
            for f in hist
                .recent_failures
                .iter()
                .rev()
                .take(self.strategy.max_recent_failures)
            {
                if let Some(ref reason) = f.reason {
                    hist_lines.push(format!("  - {} (at {})", reason, f.timestamp));
                }
            }
        }
        if let Some(ref pattern) = hist.common_failure_pattern {
            hist_lines.push(format!("- Common failure: {}", pattern));
        }
        sections.push(hist_lines.join("\n"));

        // ── 4. Ranked known issues ────────────────
        if !ranked.issues.is_empty() {
            let mut issue_lines = vec!["## Known Issues (ranked by relevance)".to_string()];
            for si in &ranked.issues {
                let trend_str = match si.trend {
                    Trend::New => "[NEW]",
                    Trend::Recurring => "[RECURRING]",
                    Trend::Worsening => "[WORSENING]",
                    Trend::Improving => "[IMPROVING]",
                    Trend::Unchanged => "",
                    Trend::RecentlyResolved => "[RESOLVED]",
                };
                issue_lines.push(format!(
                    "- {} {} ({}x) — {}",
                    trend_str,
                    si.issue.description,
                    si.issue.occurrence_count,
                    if si.issue.resolved {
                        "resolved"
                    } else {
                        "active"
                    }
                ));
            }
            sections.push(issue_lines.join("\n"));
        }

        // ── 5. Ranked recent evidence ─────────────
        if !ranked.evidence.is_empty() {
            let mut ev_lines = vec!["## Recent Evidence (ranked)".to_string()];
            for se in &ranked.evidence {
                ev_lines.push(format!(
                    "- [{}] {} (confidence: {:.2})",
                    se.evidence.quality.as_str(),
                    se.evidence.statement,
                    se.evidence.confidence
                ));
            }
            sections.push(ev_lines.join("\n"));
        }

        // ── 6. Active observations ────────────────
        if !ranked.active_observations.is_empty() {
            let mut obs_lines = vec!["## Active Observations".to_string()];
            for so in &ranked.active_observations {
                obs_lines.push(format!(
                    "- [{}] {} ({})",
                    so.observation.severity, so.observation.message, so.observation.category
                ));
            }
            sections.push(obs_lines.join("\n"));
        }

        // ── 7. Pending warnings ───────────────────
        if !context.current_state.pending_warnings.is_empty() {
            let mut warn_lines = vec!["## Pending Warnings".to_string()];
            for w in &context.current_state.pending_warnings {
                warn_lines.push(format!("- {}", w));
            }
            sections.push(warn_lines.join("\n"));
        }

        // ── 8. Contradictions ─────────────────────
        if self.strategy.include_contradictions && !contradictions.is_empty() {
            let mut contr_lines = vec!["## Contradictions Detected".to_string()];
            for c in contradictions {
                contr_lines.push(format!(
                    "- [{}] {}: `{}` vs `{}`",
                    match c.severity {
                        layermind_shared::recommendation::ContradictionSeverity::Critical =>
                            "CRITICAL",
                        layermind_shared::recommendation::ContradictionSeverity::Significant =>
                            "SIGNIFICANT",
                        layermind_shared::recommendation::ContradictionSeverity::Minor => "minor",
                    },
                    c.description,
                    c.item_a,
                    c.item_b
                ));
            }
            sections.push(contr_lines.join("\n"));
        }

        sections.join("\n\n")
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
        CurrentState, Evidence, EvidenceQuality, HealthSummary, IssueSummary, PrintHistorySummary,
        PrinterSummary,
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
        let builder = PromptBuilder::new(DiagnosticStrategy::STANDARD);
        let prompt = builder.system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("diagnostic"));
    }

    #[test]
    fn user_prompt_includes_printer_info() {
        let builder = PromptBuilder::new(DiagnosticStrategy::STANDARD);
        let ctx = basic_context();
        let prompt = builder.user_prompt(&ctx, &[]);
        assert!(prompt.contains("Test Printer"));
        assert!(prompt.contains("Ender 3 V2"));
        assert!(prompt.contains("## Printer"));
        assert!(prompt.contains("## Health"));
        assert!(prompt.contains("## Print History"));
    }

    #[test]
    fn user_prompt_includes_ranked_evidence() {
        let builder = PromptBuilder::new(DiagnosticStrategy::STANDARD);
        let mut ctx = basic_context();
        ctx.recent_evidence = vec![
            Evidence::observed("temp", "Very recent reading", 0.95, Utc::now()),
            Evidence::observed(
                "temp",
                "Old reading",
                0.5,
                Utc::now() - chrono::Duration::hours(48),
            ),
        ];
        let prompt = builder.user_prompt(&ctx, &[]);
        assert!(prompt.contains("Recent Evidence"));
        // Recent should appear before old in ranked output.
        let recent_pos = prompt.find("Very recent reading").unwrap();
        let old_pos = prompt.find("Old reading").unwrap();
        assert!(recent_pos < old_pos);
    }

    #[test]
    fn user_prompt_includes_contradictions() {
        let builder = PromptBuilder::new(DiagnosticStrategy::STANDARD);
        let ctx = basic_context();
        let contradictions = vec![Contradiction {
            description: "Test conflict".into(),
            item_a: "a".into(),
            item_b: "b".into(),
            severity: layermind_shared::recommendation::ContradictionSeverity::Significant,
        }];
        let prompt = builder.user_prompt(&ctx, &contradictions);
        assert!(prompt.contains("Contradictions Detected"));
        assert!(prompt.contains("Test conflict"));
    }

    #[test]
    fn rapid_strategy_suppresses_contradictions() {
        let builder = PromptBuilder::new(DiagnosticStrategy::RAPID);
        let ctx = basic_context();
        let contradictions = vec![Contradiction {
            description: "Test conflict".into(),
            item_a: "a".into(),
            item_b: "b".into(),
            severity: layermind_shared::recommendation::ContradictionSeverity::Significant,
        }];
        let prompt = builder.user_prompt(&ctx, &contradictions);
        assert!(!prompt.contains("Contradictions Detected"));
    }

    #[test]
    fn user_prompt_includes_historical_trends() {
        let builder = PromptBuilder::new(DiagnosticStrategy::STANDARD);
        let mut ctx = basic_context();
        ctx.known_issues = vec![IssueSummary {
            category: "thermal".into(),
            description: "PID oscillation".into(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            occurrence_count: 5,
            resolved: false,
            importance: 0.7,
        }];
        let prompt = builder.user_prompt(&ctx, &[]);
        assert!(prompt.contains("Known Issues"));
        assert!(prompt.contains("[RECURRING]"));
        assert!(prompt.contains("PID oscillation"));
    }

    #[test]
    fn prompt_pair_contains_both() {
        let builder = PromptBuilder::new(DiagnosticStrategy::STANDARD);
        let ctx = basic_context();
        let pair = builder.build(&ctx, &[]);
        assert!(!pair.system.is_empty());
        assert!(!pair.user.is_empty());
    }
}
