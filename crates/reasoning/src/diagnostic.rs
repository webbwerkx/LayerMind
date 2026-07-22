//! Print Doctor — the first AI diagnostic capability.
//!
//! Orchestrates the full reasoning pipeline:
//!   PrinterContext → PromptBuilder → AiProvider → ResponseParser
//!     → TrustValidator → ValidatedRecommendation
//!
//! This is the entry point for AI-driven printer diagnostics. Call
//! `diagnose()` with a PrinterContext and an AiProvider to get a
//! validated recommendation.

use std::sync::Arc;

use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::ValidatedRecommendation;
use tracing;

use crate::parser::parse_recommendation;
use crate::prompt::PromptBuilder;
use crate::provider::{AiProvider, AiRequest};
use crate::trust::TrustValidator;

/// AI Print Doctor — diagnoses printer issues from context.
pub struct PrintDoctor {
    provider: Arc<dyn AiProvider>,
    prompt_builder: PromptBuilder,
    trust_validator: TrustValidator,
}

impl PrintDoctor {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            prompt_builder: PromptBuilder::new(),
            trust_validator: TrustValidator::new(),
        }
    }

    /// Name of the AI provider (e.g. "openai", "mock").
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Model identifier (e.g. "gpt-4o", "mock-gpt4").
    pub fn provider_model(&self) -> &str {
        self.provider.model()
    }

    /// Run a full diagnostic: build prompts, call AI, parse response,
    /// validate trust, and return a validated recommendation.
    pub async fn diagnose(
        &self,
        context: &PrinterContext,
    ) -> Result<ValidatedRecommendation, DiagnoseError> {
        let printer_id = context.printer_id.clone();

        // 1. Build prompts.
        let pair = self.prompt_builder.build(context);

        // 2. Call AI provider.
        let request = AiRequest {
            system_prompt: pair.system,
            user_prompt: pair.user,
            max_tokens: 1024,
            temperature: 0.3, // low temp for diagnostic (want facts, not creativity)
        };

        let response = self.provider.complete(request).await.map_err(|e| {
            tracing::error!(error = %e, "AI provider failed");
            DiagnoseError::ProviderError(e.to_string())
        })?;

        // 3. Parse structured recommendation from raw response.
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

        // Attach usage tracking.
        let mut recommendation = parsed.recommendation;
        recommendation.usage = layermind_shared::recommendation::AiUsage::new(
            self.provider.name(),
            self.provider.model(),
            response.usage.prompt_tokens,
            response.usage.completion_tokens,
        );

        // 4. Validate trust.
        let trust = self.trust_validator.validate(&recommendation, context);

        // 5. Build validated recommendation.
        let validated = ValidatedRecommendation::new(recommendation, trust);

        tracing::info!(
            printer_id = %printer_id,
            category = ?validated.recommendation.category,
            severity = ?validated.recommendation.severity,
            trust = ?validated.trust.overall_trust,
            cost = validated.recommendation.usage.estimated_cost_usd,
            "diagnostic complete"
        );

        Ok(validated)
    }
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

    #[test]
    fn diagnose_with_healthy_response() {
        let mock_response = r#"{
            "category": "general",
            "severity": "info",
            "confidence": 0.95,
            "summary": "Printer is healthy with minor temperature concerns",
            "explanation": "The printer has a 93% success rate over 42 prints. Temperature stability is at 0.7 which is acceptable but could be improved. Active issue: PID oscillation on extruder (3 occurrences).",
            "actions": [
                {
                    "priority": 1,
                    "description": "Run PID calibration on extruder to improve temperature stability",
                    "suggested_command": "PID_CALIBRATE HEATER=extruder TARGET=210",
                    "expected_outcome": "Reduced temperature oscillation and improved print quality",
                    "is_safe_automatic": false
                }
            ],
            "evidence": [
                {
                    "claim": "Temperature stability is degraded",
                    "supporting_fact": "Extruder oscillating ±3.2°C around target"
                },
                {
                    "claim": "Printer has been reliable",
                    "supporting_fact": "93% success rate over 42 prints"
                }
            ]
        }"#;

        let mock = MockProvider::new("mock", "mock-model", mock_response);
        let doctor = PrintDoctor::new(Arc::new(mock));
        let ctx = test_context();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(doctor.diagnose(&ctx)).unwrap();

        assert_eq!(
            result.recommendation.category,
            RecommendationCategory::General
        );
        assert_eq!(result.recommendation.severity, Severity::Info);
        assert_eq!(result.recommendation.actions.len(), 1);
        assert!(result.recommendation.actions[0].suggested_command.is_some());
        assert!(result.recommendation.usage.estimated_cost_usd > 0.0);
    }

    #[test]
    fn diagnose_end_to_end_pipeline() {
        // Full pipeline: context → prompt → mock AI → parser → trust → validated
        let mock_json = r#"{"category":"thermal","severity":"warning","confidence":0.8,"summary":"PID tuning needed","explanation":"Extruder temperature oscillating. PID calibration recommended.","actions":[{"priority":1,"description":"Run PID_CALIBRATE","suggested_command":"PID_CALIBRATE HEATER=extruder TARGET=210","expected_outcome":"Stable temperature","is_safe_automatic":false}],"evidence":[{"claim":"Temperature is oscillating","supporting_fact":"Extruder shows 3.2°C deviation"}]}"#;
        let mock = MockProvider::new("mock", "mock-gpt4", mock_json);
        let doctor = PrintDoctor::new(Arc::new(mock));
        let ctx = test_context();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let validated = rt.block_on(doctor.diagnose(&ctx)).unwrap();

        // Verify pipeline stages:
        assert_eq!(
            validated.recommendation.category,
            RecommendationCategory::Thermal
        );
        assert_eq!(validated.recommendation.actions.len(), 1);

        // Trust assessment should find the temperature claim in context evidence.
        assert!(validated.trust.facts_cited >= 1);
        assert_eq!(validated.trust.unsupported_claims, 0);

        // Usage tracked.
        assert_eq!(validated.recommendation.usage.provider, "mock");
        assert_eq!(validated.recommendation.usage.model, "mock-gpt4");
    }
}
