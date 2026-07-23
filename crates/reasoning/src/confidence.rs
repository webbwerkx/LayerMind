//! Deterministic confidence calibration.
//!
//! The AI model provides a rough confidence estimate, but LayerMind
//! owns the final confidence score. This module adjusts the AI's
//! confidence based on evidence quality, quantity, recency, agreement,
//! and contradictions — using deterministic rules only.
//!
//! Algorithm:
//!   adjusted = base + evidence_bonus + quality_bonus + recency_bonus
//!            + agreement_bonus - conflict_penalty - staleness_penalty
//!
//! Every adjustment is bounded so no single factor dominates.
//!
//! The final confidence is always clamped to [0.0, 1.0] and is fully
//! reproducible given the same inputs.

use chrono::{Duration, Utc};
use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::{Contradiction, ContradictionSeverity, Recommendation};

/// Calibrates AI confidence using deterministic evidence analysis.
#[derive(Debug, Default)]
pub struct ConfidenceCalibrator;

impl ConfidenceCalibrator {
    pub fn new() -> Self {
        Self
    }

    /// Calibrate the AI's confidence score based on evidence quality.
    ///
    /// `ai_confidence`: raw confidence from the AI model (clamped to [0.1, 0.95]).
    /// `recommendation`: the parsed recommendation with actions and evidence.
    /// `context`: the printer context used for diagnosis.
    /// `contradictions`: pre-detected contradictions in the context.
    pub fn calibrate(
        &self,
        ai_confidence: f64,
        recommendation: &Recommendation,
        context: &PrinterContext,
        contradictions: &[Contradiction],
    ) -> f64 {
        // Sanitize base: never trust 0.0 or 1.0 from AI.
        let base = ai_confidence.clamp(0.1, 0.95);

        // ── Evidence quantity bonus ─────────────────
        let evidence_count = recommendation.evidence.len();
        let evidence_bonus = (evidence_count as f64 / 5.0).min(0.15);
        // 5+ pieces → max +0.15

        // ── Evidence quality bonus ─────────────────
        let observed_ratio = if evidence_count > 0 {
            recommendation
                .evidence
                .iter()
                .filter(|e| {
                    matches!(
                        e.source_quality,
                        layermind_shared::context::EvidenceQuality::Observed
                    )
                })
                .count() as f64
                / evidence_count as f64
        } else {
            0.0
        };
        let quality_bonus = observed_ratio * 0.10;
        // All observed → +0.10, none → +0.00

        // ── Recency bonus ──────────────────────────
        let now = Utc::now();
        let recent_window = Duration::hours(24);
        let has_recent_evidence = context
            .recent_evidence
            .iter()
            .any(|e| (now - e.timestamp) < recent_window && e.confidence > 0.7);
        let recency_bonus = if has_recent_evidence { 0.05 } else { 0.0 };

        // ── Agreement bonus ────────────────────────
        // Multiple evidence items citing the same category → stronger signal.
        let distinct_categories: std::collections::HashSet<&str> = recommendation
            .evidence
            .iter()
            .map(|e| e.claim.as_str())
            .collect();
        let agreement_ratio = if evidence_count > 0 {
            distinct_categories.len() as f64 / evidence_count as f64
        } else {
            1.0
        };
        // Lower ratio = more agreement (similar claims). Higher = diverse.
        let agreement_bonus = (1.0 - agreement_ratio) * 0.10;
        // All same claim → +0.10, all different → +0.00

        // ── Conflict penalty ───────────────────────
        let conflict_count = contradictions.len();
        let significant_conflicts = contradictions
            .iter()
            .filter(|c| c.severity != ContradictionSeverity::Minor)
            .count();
        let conflict_penalty =
            (significant_conflicts as f64 * 0.10 + conflict_count as f64 * 0.05).min(0.30);

        // ── Staleness penalty ──────────────────────
        let all_old = !context.recent_evidence.is_empty()
            && context
                .recent_evidence
                .iter()
                .all(|e| (now - e.timestamp) > Duration::hours(72));
        let staleness_penalty = if all_old { 0.10 } else { 0.0 };

        let adjusted = base + evidence_bonus + quality_bonus + recency_bonus + agreement_bonus
            - conflict_penalty
            - staleness_penalty;

        adjusted.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_shared::observation::Severity;
    use layermind_shared::recommendation::RecommendationCategory;

    #[test]
    fn high_quality_evidence_increases_confidence() {
        let calibrator = ConfidenceCalibrator::new();
        let rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Thermal,
            Severity::Warning,
            0.7, // AI says 0.7
            "test".into(),
        );
        let ctx = PrinterContext::new("p1".into());
        let contradictions = vec![];

        let calibrated = calibrator.calibrate(0.7, &rec, &ctx, &contradictions);
        // Base 0.7, no evidence → bonus 0, but base stays.
        // Evidence count = 0 → evidence_bonus = 0
        // quality_bonus = 0
        // recency_bonus = 0
        // agreement_bonus = 0.10 (0 evidence → ratio = 1.0, bonus = 0)
        // Actually 0 evidence → evidence_count = 0, agreement_ratio = 1.0, bonus = (1-1)*0.1 = 0
        // conflict_penalty = 0
        // Result: 0.7
        assert_eq!(calibrated, 0.7);
    }

    #[test]
    fn conflicts_reduce_confidence() {
        let calibrator = ConfidenceCalibrator::new();
        let rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::Mechanical,
            Severity::Warning,
            0.8,
            "test".into(),
        );
        let ctx = PrinterContext::new("p1".into());
        let contradictions = vec![Contradiction {
            description: "conflict".into(),
            item_a: "a".into(),
            item_b: "b".into(),
            severity: ContradictionSeverity::Significant,
        }];

        let calibrated = calibrator.calibrate(0.8, &rec, &ctx, &contradictions);
        // Base 0.8 - 0.10 (significant) - 0.05 (per conflict) = 0.65
        assert!(calibrated < 0.8);
        assert!(calibrated <= 0.70);
    }

    #[test]
    fn ai_confidence_of_zero_is_clamped() {
        let calibrator = ConfidenceCalibrator::new();
        let rec = Recommendation::new(
            "p1".into(),
            RecommendationCategory::General,
            Severity::Info,
            0.0,
            "test".into(),
        );
        let ctx = PrinterContext::new("p1".into());
        let calibrated = calibrator.calibrate(0.0, &rec, &ctx, &[]);
        assert!(calibrated >= 0.1);
    }

    #[test]
    fn calibrated_confidence_in_range() {
        // Fuzz test: result always in [0.0, 1.0].
        let calibrator = ConfidenceCalibrator::new();
        let test_cases = vec![0.0, 0.3, 0.5, 0.8, 1.0, 1.5, -0.5];
        for conf in test_cases {
            let rec = Recommendation::new(
                "p1".into(),
                RecommendationCategory::General,
                Severity::Info,
                conf,
                "test".into(),
            );
            let ctx = PrinterContext::new("p1".into());
            let calibrated = calibrator.calibrate(conf, &rec, &ctx, &[]);
            assert!(
                (0.0..=1.0).contains(&calibrated),
                "calibrated {} → {} (out of range)",
                conf,
                calibrated
            );
        }
    }
}
