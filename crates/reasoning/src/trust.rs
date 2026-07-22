//! Trust validator — deterministic cross-reference of AI claims against
//! context evidence.
//!
//! The validator does NOT call AI. It mechanically compares each claim
//! in the recommendation against the facts in the PrinterContext that
//! was provided to the model. It calculates a trust score, identifies
//! unsupported claims, and generates disclaimers.

use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::{Recommendation, TrustAssessment, TrustLevel};

/// Validates a recommendation against its source context.
#[derive(Debug, Default)]
pub struct TrustValidator;

impl TrustValidator {
    pub fn new() -> Self {
        Self
    }

    /// Cross-reference recommendation claims against context evidence.
    /// Returns a TrustAssessment with metrics and an overall trust level.
    pub fn validate(
        &self,
        recommendation: &Recommendation,
        context: &PrinterContext,
    ) -> TrustAssessment {
        let total_claims = recommendation.evidence.len().max(1);
        let mut facts_cited = 0usize;
        let mut inferences_made = 0usize;

        for ref_evidence in &recommendation.evidence {
            // Search context for matching evidence.
            let found_in_context = search_context(context, &ref_evidence.claim);

            match found_in_context {
                SearchResult::Observed => facts_cited += 1,
                SearchResult::Inferred => inferences_made += 1,
                SearchResult::NotFound => {} // unsupported
            }
        }

        let unsupported_claims = total_claims.saturating_sub(facts_cited + inferences_made);
        let hallucination_risk = unsupported_claims as f64 / total_claims as f64;

        let overall_trust = if hallucination_risk > 0.5 {
            TrustLevel::Low
        } else if unsupported_claims > 0 || hallucination_risk > 0.2 {
            TrustLevel::Medium
        } else {
            TrustLevel::High
        };

        TrustAssessment {
            overall_trust,
            facts_cited,
            inferences_made,
            unsupported_claims,
            hallucination_risk,
        }
    }
}

enum SearchResult {
    Observed,
    Inferred,
    NotFound,
}

/// Naive search: checks whether context evidence text contains the claim
/// keywords. In a production system this would use embeddings or semantic
/// search. For now, simple substring matching of key terms suffices as a
/// deterministic first pass.
fn search_context(context: &PrinterContext, claim: &str) -> SearchResult {
    let claim_lower = claim.to_lowercase();

    // Check recent evidence (Observed facts).
    for evidence in &context.recent_evidence {
        if text_contains_any(
            &evidence.statement.to_lowercase(),
            &extract_keywords(&claim_lower),
        ) {
            return SearchResult::Observed;
        }
    }

    // Check known issues (Inferred).
    for issue in &context.known_issues {
        if text_contains_any(
            &issue.description.to_lowercase(),
            &extract_keywords(&claim_lower),
        ) {
            return SearchResult::Inferred;
        }
    }

    // Check historical patterns (Inferred).
    for pattern in &context.historical_patterns {
        if text_contains_any(
            &pattern.description.to_lowercase(),
            &extract_keywords(&claim_lower),
        ) {
            return SearchResult::Inferred;
        }
    }

    // Check print history summary.
    if let Some(ref pattern) = context.print_history.common_failure_pattern {
        if text_contains_any(&pattern.to_lowercase(), &extract_keywords(&claim_lower)) {
            return SearchResult::Inferred;
        }
    }

    // Check current state warnings.
    for warning in &context.current_state.pending_warnings {
        if text_contains_any(&warning.to_lowercase(), &extract_keywords(&claim_lower)) {
            return SearchResult::Inferred;
        }
    }

    SearchResult::NotFound
}

/// Extract meaningful keywords from a claim for matching.
fn extract_keywords(text: &str) -> Vec<String> {
    // Split on whitespace and punctuation, filter short words.
    text.split(|c: char| c.is_whitespace() || c == ',' || c == '.' || c == ':')
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_string())
        .collect()
}

/// Check if text contains any of the given keywords.
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
    use layermind_shared::recommendation::{RecommendationCategory, Reference};

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

        let mut recommendation = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Thermal,
            Severity::Warning,
            0.85,
            "Temperature unstable".into(),
        );
        recommendation.evidence.push(Reference {
            claim: "Temperature is oscillating".into(),
            supporting_fact: "Extruder shows ±3°C oscillation".into(),
            source_quality: EvidenceQuality::Observed,
            source_id: None,
        });

        let assessment = validator.validate(&recommendation, &ctx);
        assert_eq!(assessment.overall_trust, TrustLevel::High);
        assert_eq!(assessment.facts_cited, 1);
        assert_eq!(assessment.unsupported_claims, 0);
        assert!(assessment.hallucination_risk < 0.01);
    }

    #[test]
    fn unsupported_claim_yields_low_trust() {
        let validator = TrustValidator::new();
        let ctx = test_context();

        let mut recommendation = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Mechanical,
            Severity::Warning,
            0.7,
            "Belt tension".into(),
        );
        recommendation.evidence.push(Reference {
            claim: "X-axis belt is loose".into(),
            supporting_fact: "Visible sag in belt".into(),
            source_quality: EvidenceQuality::Inferred,
            source_id: None,
        });

        let assessment = validator.validate(&recommendation, &ctx);
        assert_eq!(assessment.overall_trust, TrustLevel::Low);
        assert_eq!(assessment.unsupported_claims, 1);
        assert!(assessment.hallucination_risk > 0.5);
    }

    #[test]
    fn mixed_claims_yield_medium_trust() {
        let validator = TrustValidator::new();
        let ctx = test_context();

        let mut recommendation = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Thermal,
            Severity::Warning,
            0.8,
            "Multiple issues".into(),
        );
        recommendation.evidence.push(Reference {
            claim: "Temperature is oscillating".into(),
            supporting_fact: "Extruder oscillation".into(),
            source_quality: EvidenceQuality::Observed,
            source_id: None,
        });
        recommendation.evidence.push(Reference {
            claim: "Belt is loose".into(),
            supporting_fact: "Visible sag".into(),
            source_quality: EvidenceQuality::Inferred,
            source_id: None,
        });

        let assessment = validator.validate(&recommendation, &ctx);
        assert_eq!(assessment.overall_trust, TrustLevel::Medium);
        assert_eq!(assessment.facts_cited, 1);
        assert_eq!(assessment.unsupported_claims, 1);
    }

    #[test]
    fn empty_evidence_still_validates() {
        let validator = TrustValidator::new();
        let ctx = test_context();

        let recommendation = Recommendation::new(
            "p1".into(),
            RecommendationCategory::General,
            Severity::Info,
            1.0,
            "No issues".into(),
        );

        let assessment = validator.validate(&recommendation, &ctx);
        // total_claims is max(0, 1) = 1, all unsupported → low trust
        assert_eq!(assessment.overall_trust, TrustLevel::Low);
        assert_eq!(assessment.unsupported_claims, 1);
    }
}
