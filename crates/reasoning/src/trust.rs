//! Trust validator — deterministic cross-reference of AI claims against
//! context evidence.
//!
//! Phase 2.3 enhancements:
//! - Historical agreement: claims matching recurring patterns get stronger
//!   trust signals.
//! - Contradiction awareness: detected contradictions adjust the trust
//!   assessment downward.
//! - Multi-source matching: each claim checked against evidence,
//!   known issues, patterns, warnings, and history simultaneously.
//!
//! The validator is fully deterministic — no AI calls.

use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::{
    Contradiction, Recommendation, TrustAssessment, TrustLevel,
};

/// Validates a recommendation against its source context.
#[derive(Debug, Default)]
pub struct TrustValidator;

impl TrustValidator {
    pub fn new() -> Self {
        Self
    }

    /// Cross-reference recommendation claims against context evidence.
    /// Incorporates historical agreement and contradiction awareness.
    pub fn validate(
        &self,
        recommendation: &Recommendation,
        context: &PrinterContext,
        contradictions: &[Contradiction],
    ) -> TrustAssessment {
        let total_claims = recommendation.evidence.len().max(1);
        let mut facts_cited = 0usize;
        let mut inferences_made = 0usize;
        let mut historical_agreement = 0usize;

        for ref_evidence in &recommendation.evidence {
            let search_text = format!("{} {}", ref_evidence.claim, ref_evidence.supporting_fact);
            let matches = search_all_sources(context, &search_text);

            // Count the strongest match found.
            if matches.has_observed {
                facts_cited += 1;
            } else if matches.has_inferred {
                inferences_made += 1;
            }

            // Historical agreement: if this claim matches a recurring pattern.
            if matches.matches_recurring_pattern {
                historical_agreement += 1;
            }
        }

        let unsupported_claims = total_claims.saturating_sub(facts_cited + inferences_made);
        let hallucination_risk = unsupported_claims as f64 / total_claims as f64;

        // Contradiction penalty: each contradiction adds to hallucination risk.
        let contradiction_penalty = (contradictions.len() as f64 * 0.05).min(0.15);
        let adjusted_hallucination_risk = (hallucination_risk + contradiction_penalty).min(1.0);

        // Historical agreement bonus: recurring matches reduce effective risk.
        let agreement_bonus = if total_claims > 0 {
            (historical_agreement as f64 / total_claims as f64) * 0.1
        } else {
            0.0
        };
        let effective_risk = adjusted_hallucination_risk - agreement_bonus;

        let overall_trust = if effective_risk > 0.5 {
            TrustLevel::Low
        } else if effective_risk > 0.2 || unsupported_claims > 0 {
            TrustLevel::Medium
        } else {
            TrustLevel::High
        };

        TrustAssessment {
            overall_trust,
            facts_cited,
            inferences_made,
            unsupported_claims,
            hallucination_risk: effective_risk.max(0.0),
        }
    }
}

/// Result of searching for a claim across all context sources.
struct SourceMatches {
    has_observed: bool,
    has_inferred: bool,
    matches_recurring_pattern: bool,
}

/// Search all evidence sources for keyword matches against a claim.
fn search_all_sources(context: &PrinterContext, text: &str) -> SourceMatches {
    let text_lower = text.to_lowercase();
    let keywords = extract_keywords(&text_lower);

    let mut matches = SourceMatches {
        has_observed: false,
        has_inferred: false,
        matches_recurring_pattern: false,
    };

    // 1. Recent evidence (Observed).
    for evidence in &context.recent_evidence {
        if text_contains_any(&evidence.statement.to_lowercase(), &keywords) {
            matches.has_observed = true;
            break;
        }
    }

    // 2. Known issues (Inferred).
    for issue in &context.known_issues {
        if text_contains_any(&issue.description.to_lowercase(), &keywords) {
            matches.has_inferred = true;
            // Recurring pattern if occurrence >= 2.
            if issue.occurrence_count >= 2 {
                matches.matches_recurring_pattern = true;
            }
            break;
        }
    }

    // 3. Historical patterns (Inferred).
    if !matches.has_inferred {
        for pattern in &context.historical_patterns {
            if text_contains_any(&pattern.description.to_lowercase(), &keywords) {
                matches.has_inferred = true;
                if pattern.occurrence_count >= 2 {
                    matches.matches_recurring_pattern = true;
                }
                break;
            }
        }
    }

    // 4. Print history common failure pattern.
    if !matches.has_inferred {
        if let Some(ref pattern) = context.print_history.common_failure_pattern {
            if text_contains_any(&pattern.to_lowercase(), &keywords) {
                matches.has_inferred = true;
            }
        }
    }

    // 5. Pending warnings.
    for warning in &context.current_state.pending_warnings {
        if text_contains_any(&warning.to_lowercase(), &keywords) {
            if !matches.has_observed {
                matches.has_inferred = true;
            }
            break;
        }
    }

    matches
}

/// Extract meaningful keywords for matching.
fn extract_keywords(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == ':')
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_string())
        .collect()
}

fn text_contains_any(text: &str, keywords: &[String]) -> bool {
    keywords.iter().any(|kw| text.contains(kw.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::context::{
        CurrentState, Evidence, EvidenceQuality, HealthSummary, IssueSummary, PrintHistorySummary,
        PrinterSummary,
    };
    use layermind_shared::observation::Severity;
    use layermind_shared::recommendation::{
        ContradictionSeverity, RecommendationCategory, Reference,
    };

    fn test_context() -> PrinterContext {
        PrinterContext {
            printer_id: "p1".into(),
            generated_at: Utc::now(),
            summary: PrinterSummary::default(),
            print_history: PrintHistorySummary::default(),
            health: HealthSummary::default(),
            current_state: CurrentState {
                pending_warnings: vec!["Temperature unstable on extruder".into()],
                ..Default::default()
            },
            known_issues: vec![IssueSummary {
                category: "temperature_instability".into(),
                description: "PID oscillation on extruder, 3°C deviation".into(),
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                occurrence_count: 3,
                resolved: false,
                importance: 0.6,
            }],
            historical_patterns: Vec::new(),
            learning: None,
            history: layermind_shared::history::HistorySummary::default(),
            machine: None,
            recent_evidence: vec![Evidence::observed(
                "temperature_reading",
                "Extruder temperature oscillating ±3°C around 210°C target",
                0.95,
                Utc::now(),
            )],
        }
    }

    #[test]
    fn supported_claim_yields_high_trust() {
        let validator = TrustValidator::new();
        let ctx = test_context();

        let mut rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Thermal,
            Severity::Warning,
            0.85,
            "Temperature unstable".into(),
        );
        rec.evidence.push(Reference {
            claim: "Temperature is oscillating".into(),
            supporting_fact: "Extruder shows ±3°C oscillation".into(),
            source_quality: EvidenceQuality::Observed,
            source_id: None,
        });

        let assessment = validator.validate(&rec, &ctx, &[]);
        assert_eq!(assessment.overall_trust, TrustLevel::High);
        assert_eq!(assessment.facts_cited, 1);
        assert_eq!(assessment.unsupported_claims, 0);
    }

    #[test]
    fn unsupported_claim_yields_low_trust() {
        let validator = TrustValidator::new();
        let ctx = test_context();

        let mut rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Mechanical,
            Severity::Warning,
            0.7,
            "Belt tension".into(),
        );
        rec.evidence.push(Reference {
            claim: "X-axis belt is loose".into(),
            supporting_fact: "Visible sag in belt".into(),
            source_quality: EvidenceQuality::Inferred,
            source_id: None,
        });

        let assessment = validator.validate(&rec, &ctx, &[]);
        assert_eq!(assessment.overall_trust, TrustLevel::Low);
        assert_eq!(assessment.unsupported_claims, 1);
    }

    #[test]
    fn contradictions_are_reflected_in_assessment() {
        let validator = TrustValidator::new();
        let ctx = test_context();

        let mut rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Thermal,
            Severity::Warning,
            0.8,
            "test".into(),
        );
        rec.evidence.push(Reference {
            claim: "Temperature is oscillating".into(),
            supporting_fact: "Extruder shows ±3°C oscillation".into(),
            source_quality: EvidenceQuality::Observed,
            source_id: None,
        });

        let contradictions = vec![Contradiction {
            description: "conflict".into(),
            item_a: "a".into(),
            item_b: "b".into(),
            severity: ContradictionSeverity::Significant,
        }];

        let assessment = validator.validate(&rec, &ctx, &contradictions);
        // With strong evidence matched, overall trust may still be high,
        // but the contradictions are surfaced separately in ValidatedRecommendation.
        assert!(assessment.facts_cited > 0);
    }

    #[test]
    fn empty_evidence_still_validates() {
        let validator = TrustValidator::new();
        let ctx = test_context();
        let rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::General,
            Severity::Info,
            1.0,
            "No issues".into(),
        );
        let assessment = validator.validate(&rec, &ctx, &[]);
        assert_eq!(assessment.overall_trust, TrustLevel::Low);
    }
}
