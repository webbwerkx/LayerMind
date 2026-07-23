//! Evidence ranking — scores and ranks PrinterContext evidence for
//! prompt generation.
//!
//! Before sending context to the AI, evidence is ranked by relevance
//! so the most useful facts appear first. Lower-quality, redundant, or
//! stale evidence is deprioritized to keep prompts concise.
//!
//! Scoring formula (deterministic, no AI):
//!   score = recency * 0.30 + confidence * 0.25 + repetition * 0.20
//!         + severity * 0.15 + active * 0.10
//!
//! Higher score = more relevant for AI diagnosis.

use crate::strategy::DiagnosticStrategy;
use chrono::Utc;
use layermind_shared::context::{Evidence, IssueSummary, ObservationSummary, PrinterContext};

/// Ranked collections for prompt injection.
#[derive(Debug, Clone)]
pub struct RankedContext {
    pub evidence: Vec<ScoredEvidence>,
    pub issues: Vec<ScoredIssue>,
    pub active_observations: Vec<ScoredObservation>,
}

/// A piece of evidence with its relevance score.
#[derive(Debug, Clone)]
pub struct ScoredEvidence {
    pub evidence: Evidence,
    pub score: f64,
}

/// A known issue with its relevance score.
#[derive(Debug, Clone)]
pub struct ScoredIssue {
    pub issue: IssueSummary,
    pub score: f64,
    /// Historical trend: recurring, worsening, improving, etc.
    pub trend: layermind_shared::recommendation::Trend,
}

/// An active observation with its relevance score.
#[derive(Debug, Clone)]
pub struct ScoredObservation {
    pub observation: ObservationSummary,
    pub score: f64,
}

/// Ranks evidence, issues, and observations from a PrinterContext.
pub struct EvidenceRanker {
    strategy: DiagnosticStrategy,
}

impl EvidenceRanker {
    pub fn new(strategy: DiagnosticStrategy) -> Self {
        Self { strategy }
    }

    /// Rank all evidence sources from the context. Returns top-N
    /// results sorted by score descending.
    pub fn rank(&self, context: &PrinterContext) -> RankedContext {
        let now = Utc::now();
        let mut scored_evidence = self.score_evidence(&context.recent_evidence, now);
        let mut scored_issues = self.score_issues(&context.known_issues, now);
        let mut scored_observations =
            self.score_observations(&context.current_state.active_observations, now);

        scored_evidence.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored_issues.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored_observations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        scored_evidence.truncate(self.strategy.max_evidence);
        scored_issues.truncate(self.strategy.max_issues);
        scored_observations.truncate(self.strategy.max_observations);

        RankedContext {
            evidence: scored_evidence,
            issues: scored_issues,
            active_observations: scored_observations,
        }
    }

    fn score_evidence(
        &self,
        evidence: &[Evidence],
        now: chrono::DateTime<Utc>,
    ) -> Vec<ScoredEvidence> {
        evidence
            .iter()
            .map(|e| {
                let age_hours = (now - e.timestamp).num_hours().max(0) as f64;
                let recency = (-age_hours / 24.0).exp();
                let confidence = e.confidence;
                let score = recency * 0.30 + confidence * 0.25 + 0.2 + 0.15 + 0.10;
                ScoredEvidence {
                    evidence: e.clone(),
                    score,
                }
            })
            .collect()
    }

    fn score_issues(
        &self,
        issues: &[IssueSummary],
        now: chrono::DateTime<Utc>,
    ) -> Vec<ScoredIssue> {
        issues
            .iter()
            .map(|i| {
                let age_hours = (now - i.last_seen).num_hours().max(0) as f64;
                let recency = (-age_hours / 24.0).exp();
                let confidence = if i.resolved { 0.3 } else { 0.8 };
                let severity = if i.resolved { 0.0 } else { i.importance };
                let repetition = 1.0 - (-(i.occurrence_count as f64) / 3.0).exp();
                let active = if i.resolved { 0.0 } else { 1.0 };

                let score = recency * 0.30
                    + confidence * 0.25
                    + repetition * 0.20
                    + severity * 0.15
                    + active * 0.10;

                let trend = if i.resolved {
                    layermind_shared::recommendation::Trend::RecentlyResolved
                } else if i.occurrence_count == 1 {
                    layermind_shared::recommendation::Trend::New
                } else if i.occurrence_count >= 3 {
                    layermind_shared::recommendation::Trend::Recurring
                } else {
                    layermind_shared::recommendation::Trend::Unchanged
                };

                ScoredIssue {
                    issue: i.clone(),
                    score,
                    trend,
                }
            })
            .collect()
    }

    fn score_observations(
        &self,
        observations: &[ObservationSummary],
        now: chrono::DateTime<Utc>,
    ) -> Vec<ScoredObservation> {
        observations
            .iter()
            .map(|o| {
                let age_hours = (now - o.timestamp).num_hours().max(0) as f64;
                let recency = (-age_hours / 24.0).exp();
                let confidence = o.confidence;
                let severity = severity_to_score(&o.severity);
                let score = recency * 0.30 + confidence * 0.25 + 0.2 + severity * 0.15 + 0.10;
                ScoredObservation {
                    observation: o.clone(),
                    score,
                }
            })
            .collect()
    }
}

fn severity_to_score(severity: &str) -> f64 {
    match severity {
        "critical" => 1.0,
        "warning" => 0.6,
        _ => 0.3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_shared::context::{
        CurrentState, EvidenceQuality, HealthSummary, PrintHistorySummary, PrinterSummary,
    };

    fn context_with_evidence(evidence: Vec<Evidence>) -> PrinterContext {
        PrinterContext {
            printer_id: "p1".into(),
            generated_at: Utc::now(),
            summary: PrinterSummary::default(),
            print_history: PrintHistorySummary::default(),
            health: HealthSummary::default(),
            current_state: CurrentState::default(),
            known_issues: Vec::new(),
            historical_patterns: Vec::new(),
            learning: None,
            history: layermind_shared::history::HistorySummary::default(),
            machine: None,
            recent_evidence: evidence,
        }
    }

    #[test]
    fn recent_evidence_ranks_higher() {
        let old = Evidence::observed(
            "temp",
            "old reading",
            0.9,
            Utc::now() - chrono::Duration::hours(48),
        );
        let recent = Evidence::observed("temp", "recent reading", 0.9, Utc::now());
        let ctx = context_with_evidence(vec![old, recent]);
        let ranker = EvidenceRanker::new(DiagnosticStrategy::STANDARD);
        let ranked = ranker.rank(&ctx);
        assert_eq!(ranked.evidence.len(), 2);
        assert!(ranked.evidence[0].score > ranked.evidence[1].score);
    }

    #[test]
    fn high_confidence_ranks_higher() {
        let low_conf = Evidence::observed("temp", "reading", 0.3, Utc::now());
        let high_conf = Evidence::observed("temp", "reading", 0.95, Utc::now());
        let ctx = context_with_evidence(vec![low_conf, high_conf]);
        let ranker = EvidenceRanker::new(DiagnosticStrategy::STANDARD);
        let ranked = ranker.rank(&ctx);
        assert!(ranked.evidence[0].score > ranked.evidence[1].score);
    }

    #[test]
    fn empty_context_produces_empty_rankings() {
        let ctx = context_with_evidence(Vec::new());
        let ranker = EvidenceRanker::new(DiagnosticStrategy::STANDARD);
        let ranked = ranker.rank(&ctx);
        assert!(ranked.evidence.is_empty());
        assert!(ranked.issues.is_empty());
        assert!(ranked.active_observations.is_empty());
    }

    #[test]
    fn truncated_to_limits() {
        let evidence: Vec<Evidence> = (0..25)
            .map(|i| Evidence::observed("temp", &format!("reading {}", i), 0.5, Utc::now()))
            .collect();
        let ctx = context_with_evidence(evidence);
        let ranker = EvidenceRanker::new(DiagnosticStrategy::STANDARD);
        let ranked = ranker.rank(&ctx);
        assert_eq!(
            ranked.evidence.len(),
            DiagnosticStrategy::STANDARD.max_evidence
        );
    }

    #[test]
    fn rapid_strategy_limits_are_smaller() {
        let evidence: Vec<Evidence> = (0..30)
            .map(|i| Evidence::observed("temp", &format!("reading {}", i), 0.5, Utc::now()))
            .collect();
        let ctx = context_with_evidence(evidence);
        let ranker = EvidenceRanker::new(DiagnosticStrategy::RAPID);
        let ranked = ranker.rank(&ctx);
        assert_eq!(
            ranked.evidence.len(),
            DiagnosticStrategy::RAPID.max_evidence
        );
    }
}
