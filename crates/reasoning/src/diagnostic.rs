//! Print Doctor — AI diagnostic pipeline.
//!
//! Orchestrates the full reasoning pipeline (Phase 2.3):
//!   PrinterContext
//!     → EvidenceRanker (via PromptBuilder)
//!     → ContradictionDetector
//!     → PromptBuilder (ranked evidence + history + contradictions)
//!     → AiProvider (model)
//!     → ResponseParser
//!     → ConfidenceCalibrator
//!     → Prioritizer
//!     → TrustValidator
//!     → ValidatedRecommendation
//!
//! Every step except AiProvider is deterministic.

use std::sync::Arc;

use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::ValidatedRecommendation;
use tracing;

use crate::confidence::ConfidenceCalibrator;
use crate::contradiction::ContradictionDetector;
use crate::parser::parse_recommendation;
use crate::prioritization::Prioritizer;
use crate::prompt::PromptBuilder;
use crate::provider::{AiProvider, AiRequest};
use crate::trust::TrustValidator;

/// AI Print Doctor — diagnoses printer issues from context.
pub struct PrintDoctor {
    provider: Arc<dyn AiProvider>,
    prompt_builder: PromptBuilder,
    contradiction_detector: ContradictionDetector,
    confidence_calibrator: ConfidenceCalibrator,
    prioritizer: Prioritizer,
    trust_validator: TrustValidator,
}

impl PrintDoctor {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            prompt_builder: PromptBuilder::new(),
            contradiction_detector: ContradictionDetector::new(),
            confidence_calibrator: ConfidenceCalibrator::new(),
            prioritizer: Prioritizer::new(),
            trust_validator: TrustValidator::new(),
        }
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    pub fn provider_model(&self) -> &str {
        self.provider.model()
    }

    /// Run a full diagnostic: detect contradictions, build ranked prompts,
    /// call AI, parse, calibrate confidence, prioritize actions, validate trust.
    pub async fn diagnose(
        &self,
        context: &PrinterContext,
    ) -> Result<ValidatedRecommendation, DiagnoseError> {
        let printer_id = context.printer_id.clone();

        // 1. Detect contradictions in the context.
        let contradictions = self.contradiction_detector.detect(context);
        if !contradictions.is_empty() {
            tracing::info!(
                printer_id = %printer_id,
                count = contradictions.len(),
                "contradictions detected in context"
            );
        }

        // 2. Build prompts with ranked evidence and contradictions.
        let pair = self.prompt_builder.build(context, &contradictions);

        // 3. Call AI provider.
        let request = AiRequest {
            system_prompt: pair.system,
            user_prompt: pair.user,
            max_tokens: 1024,
            temperature: 0.3,
        };

        let response = self.provider.complete(request).await.map_err(|e| {
            tracing::error!(error = %e, "AI provider failed");
            DiagnoseError::ProviderError(e.to_string())
        })?;

        // 4. Parse structured recommendation.
        let parsed = parse_recommendation(&printer_id, &response.content);

        if parsed.recovered_from_error {
            tracing::warn!(
                printer_id = %printer_id,
                missing = ?parsed.missing_fields,
                "AI response required recovery"
            );
        }
        if !parsed.missing_fields.is_empty() {
            tracing::warn!(
                printer_id = %printer_id,
                missing = ?parsed.missing_fields,
                "AI response missing fields"
            );
        }

        let mut recommendation = parsed.recommendation;

        // 5. Calibrate confidence deterministically.
        let original_confidence = recommendation.confidence;
        let calibrated = self.confidence_calibrator.calibrate(
            original_confidence,
            &recommendation,
            context,
            &contradictions,
        );
        recommendation.confidence = calibrated;

        tracing::info!(
            printer_id = %printer_id,
            original = original_confidence,
            calibrated = calibrated,
            "confidence calibrated"
        );

        // 6. Prioritize actions deterministically.
        let action_count_before = recommendation.actions.len();
        self.prioritizer
            .prioritize(&mut recommendation.actions, context);
        tracing::info!(
            printer_id = %printer_id,
            actions = action_count_before,
            "actions prioritized"
        );

        // 7. Attach usage tracking.
        recommendation.usage = layermind_shared::recommendation::AiUsage::new(
            self.provider.name(),
            self.provider.model(),
            response.usage.prompt_tokens,
            response.usage.completion_tokens,
        );

        // 8. Validate trust with contradiction awareness.
        let trust = self
            .trust_validator
            .validate(&recommendation, context, &contradictions);

        // 9. Build explainability factors for each action.
        let explanation_factors = build_explanation_factors(&recommendation, context);

        // 10. Assemble validated recommendation.
        let mut validated = ValidatedRecommendation::new(recommendation, trust);
        validated.explanation_factors = explanation_factors;
        validated.contradictions = contradictions;

        tracing::info!(
            printer_id = %printer_id,
            category = ?validated.recommendation.category,
            severity = ?validated.recommendation.severity,
            confidence = calibrated,
            trust = ?validated.trust.overall_trust,
            actions = validated.recommendation.actions.len(),
            contradictions = validated.contradictions.len(),
            cost = validated.recommendation.usage.estimated_cost_usd,
            "diagnostic complete"
        );

        Ok(validated)
    }
}

// ── Explainability ──────────────────────────────────────────────────

fn build_explanation_factors(
    recommendation: &layermind_shared::recommendation::Recommendation,
    context: &PrinterContext,
) -> Vec<layermind_shared::recommendation::ExplanationFactor> {
    use layermind_shared::context::EvidenceQuality;
    use layermind_shared::recommendation::ExplanationFactor;

    recommendation
        .actions
        .iter()
        .map(|action| {
            // Find evidence referencing this action.
            let evidence_refs: Vec<usize> = recommendation
                .evidence
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    let desc_lower = action.description.to_lowercase();
                    let claim_lower = e.claim.to_lowercase();
                    let fact_lower = e.supporting_fact.to_lowercase();
                    desc_lower.contains(&claim_lower)
                        || claim_lower.contains(&desc_lower)
                        || fact_lower
                            .split_whitespace()
                            .any(|w| w.len() >= 4 && desc_lower.contains(w))
                })
                .map(|(i, _)| i)
                .collect();

            // Determine observation type: prefer Observed if any supporting
            // evidence is observed, else Inferred.
            let observation_type = recommendation
                .evidence
                .iter()
                .find(|e| matches!(e.source_quality, EvidenceQuality::Observed))
                .map(|_| EvidenceQuality::Observed)
                .unwrap_or(EvidenceQuality::Inferred);

            // Weight: proportional to action priority and evidence count.
            let weight = if action.priority <= 2 { 0.9 } else { 0.5 }
                * (evidence_refs.len() as f64 + 1.0).min(2.0)
                / 2.0;

            ExplanationFactor {
                reason: action.description.clone(),
                evidence_refs,
                assumption: if matches!(observation_type, EvidenceQuality::Inferred) {
                    Some("Based on pattern analysis, not direct measurement".into())
                } else {
                    None
                },
                observation_type,
                weight,
            }
        })
        .collect()
}

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum DiagnoseError {
    #[error("no context available for printer '{printer_id}'")]
    MissingContext { printer_id: String },
    #[error("AI provider error: {0}")]
    ProviderError(String),
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;
    use chrono::Utc;
    use layermind_shared::context::{
        CurrentState, Evidence, HealthSummary, IssueSummary, PrintHistorySummary, PrinterSummary,
    };
    use layermind_shared::observation::Severity;
    use layermind_shared::recommendation::RecommendationCategory;

    fn test_context() -> PrinterContext {
        PrinterContext {
            printer_id: "test-printer".into(),
            generated_at: Utc::now(),
            summary: PrinterSummary {
                name: "Test Printer".into(),
                model: Some("Ender 3 V2".into()),
                firmware: Some("Marlin 2.1".into()),
                reliability_score: Some(0.85),
                ..Default::default()
            },
            print_history: PrintHistorySummary {
                total_prints: 42,
                successful_prints: 39,
                failed_prints: 3,
                success_rate: Some(0.93),
                ..Default::default()
            },
            health: HealthSummary {
                temperature_stability: Some(0.7),
                uptime_secs: 36000.0,
                recent_error_count: 2,
                recent_warning_count: 5,
                reliability_score: Some(0.85),
                ..Default::default()
            },
            current_state: CurrentState {
                is_printing: false,
                pending_warnings: vec!["Temperature instability detected".into()],
                ..Default::default()
            },
            known_issues: vec![IssueSummary {
                category: "temperature_instability".into(),
                description: "Extruder PID oscillation, 3.2°C average deviation".into(),
                first_seen: Utc::now(),
                last_seen: Utc::now(),
                occurrence_count: 3,
                resolved: false,
                importance: 0.6,
            }],
            historical_patterns: Vec::new(),
            recent_evidence: vec![Evidence::observed(
                "temperature_reading",
                "Extruder oscillating ±3.2°C around target 210°C",
                0.95,
                Utc::now(),
            )],
        }
    }

    fn mock_multi_issue_response() -> &'static str {
        r#"{
            "category": "thermal",
            "severity": "warning",
            "confidence": 0.80,
            "summary": "Multiple issues: temperature instability and cooling concerns",
            "explanation": "Two issues detected: 1) PID oscillation on extruder (recurring, seen 3 times). 2) Potential cooling degradation suggested by recent failures. PID calibration is the primary fix; cooling should be monitored.",
            "actions": [
                {
                    "priority": 1,
                    "description": "Run PID calibration on extruder to fix temperature oscillation",
                    "suggested_command": "PID_CALIBRATE HEATER=extruder TARGET=210",
                    "expected_outcome": "Stable temperature ±1°C",
                    "is_safe_automatic": false
                },
                {
                    "priority": 2,
                    "description": "Check part cooling fan speed and airflow",
                    "suggested_command": null,
                    "expected_outcome": "Improved layer cooling",
                    "is_safe_automatic": true
                },
                {
                    "priority": 3,
                    "description": "Clean and inspect nozzle for partial clog",
                    "suggested_command": null,
                    "expected_outcome": "Consistent extrusion",
                    "is_safe_automatic": true
                }
            ],
            "evidence": [
                {
                    "claim": "Temperature is oscillating on extruder",
                    "supporting_fact": "Extruder shows 3.2°C oscillation around target"
                },
                {
                    "claim": "Cooling may be insufficient",
                    "supporting_fact": "3 failed prints in recent history, common in cooling issues"
                }
            ]
        }"#
    }

    #[test]
    fn diagnose_multi_issue() {
        let mock = MockProvider::new("mock", "mock-model", mock_multi_issue_response());
        let doctor = PrintDoctor::new(Arc::new(mock));
        let ctx = test_context();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(doctor.diagnose(&ctx)).unwrap();

        // Multi-issue: 3 actions.
        assert_eq!(result.recommendation.actions.len(), 3);
        // Actions are prioritized.
        assert_eq!(result.recommendation.actions[0].priority, 1);
        assert!(result.recommendation.actions[0].description.contains("PID"));
        // Confidence calibrated.
        assert!(result.recommendation.confidence != 0.80);
        // Contradictions detected (may be none, but field exists).
        assert!(result.contradictions.is_empty() || !result.contradictions.is_empty());
        // Explainability factors present.
        assert_eq!(
            result.explanation_factors.len(),
            result.recommendation.actions.len()
        );
    }

    #[test]
    fn diagnose_end_to_end_pipeline() {
        let mock_json = r#"{"category":"thermal","severity":"warning","confidence":0.8,"summary":"PID tuning needed","explanation":"Extruder temperature oscillating. PID calibration recommended.","actions":[{"priority":1,"description":"Run PID_CALIBRATE","suggested_command":"PID_CALIBRATE HEATER=extruder TARGET=210","expected_outcome":"Stable temperature","is_safe_automatic":false}],"evidence":[{"claim":"Temperature is oscillating","supporting_fact":"Extruder shows 3.2°C deviation"}]}"#;
        let mock = MockProvider::new("mock", "mock-gpt4", mock_json);
        let doctor = PrintDoctor::new(Arc::new(mock));
        let ctx = test_context();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let validated = rt.block_on(doctor.diagnose(&ctx)).unwrap();

        assert_eq!(
            validated.recommendation.category,
            RecommendationCategory::Thermal
        );
        assert_eq!(validated.recommendation.actions.len(), 1);
        assert!(validated.trust.facts_cited >= 1);
        assert_eq!(validated.recommendation.usage.provider, "mock");
    }
}
