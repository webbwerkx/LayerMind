//! Context Engine — subscribes to Knowledge stream, builds cached
//! printer context, and produces AI-ready briefings on demand.

use std::collections::HashMap;

use chrono::Utc;
use layermind_shared::context::{
    CurrentState, Evidence, EvidenceQuality, HealthSummary, HistoricalPattern, IssueSummary,
    ObservationSummary, PrintHistorySummary, PrinterContext, PrinterSummary, RecentFailure,
};
use layermind_shared::knowledge::{Knowledge, KnowledgeKind};
use layermind_shared::observation::{ObservationKind, Severity};
use tokio::sync::broadcast;
use tracing;

/// How many pieces of evidence to keep in the context.
const MAX_EVIDENCE: usize = 20;

/// How many recent events to track for current state.
const MAX_RECENT_EVENTS: usize = 10;

/// How many recent failures to include in history.
const MAX_RECENT_FAILURES: usize = 5;

/// Caches knowledge state and produces PrinterContext on demand.
pub struct ContextEngine {
    rx: broadcast::Receiver<Knowledge>,
    printers: HashMap<String, CachedContext>,
}

#[derive(Debug, Clone)]
struct CachedContext {
    printer_id: String,
    // PrinterSummary fields
    name: String,
    model: Option<String>,
    firmware: Option<String>,
    first_seen: Option<chrono::DateTime<Utc>>,
    last_seen: Option<chrono::DateTime<Utc>>,
    reliability_score: Option<f64>,
    total_observations: u64,
    total_prints: u64,
    // PrintHistorySummary fields
    successful_prints: u64,
    failed_prints: u64,
    avg_duration_secs: Option<f64>,
    recent_failures: Vec<RecentFailure>,
    // HealthSummary fields
    temperature_stability: Option<f64>,
    success_rate: Option<f64>,
    uptime_secs: f64,
    recent_error_count: u64,
    recent_warning_count: u64,
    // CurrentState fields
    is_printing: bool,
    active_print_filename: Option<String>,
    active_observations: Vec<ObservationSummary>,
    pending_warnings: Vec<String>,
    // Known issues
    known_issues: Vec<IssueSummary>,
    // Historical patterns
    patterns: Vec<HistoricalPattern>,
    // Evidence ledger
    evidence: Vec<Evidence>,
}

impl CachedContext {
    fn new(printer_id: String) -> Self {
        Self {
            printer_id,
            name: String::new(),
            model: None,
            firmware: None,
            first_seen: None,
            last_seen: None,
            reliability_score: None,
            total_observations: 0,
            total_prints: 0,
            successful_prints: 0,
            failed_prints: 0,
            avg_duration_secs: None,
            recent_failures: Vec::new(),
            temperature_stability: None,
            success_rate: None,
            uptime_secs: 0.0,
            recent_error_count: 0,
            recent_warning_count: 0,
            is_printing: false,
            active_print_filename: None,
            active_observations: Vec::new(),
            pending_warnings: Vec::new(),
            known_issues: Vec::new(),
            patterns: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

impl ContextEngine {
    pub fn new(rx: broadcast::Receiver<Knowledge>) -> Self {
        Self {
            rx,
            printers: HashMap::new(),
        }
    }

    /// Run the context engine — consume knowledge records and update
    /// cached state. Runs until the broadcast sender is dropped.
    pub async fn run(mut self) {
        tracing::info!("context engine starting");

        loop {
            match self.rx.recv().await {
                Ok(knowledge) => {
                    self.process(knowledge);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "context engine lagging");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        tracing::info!("context engine stopped");
    }

    /// Produce the current context for a printer.
    pub fn context(&self, printer_id: &str) -> Option<PrinterContext> {
        let cached = self.printers.get(printer_id)?;

        let health = HealthSummary {
            temperature_stability: cached.temperature_stability,
            success_rate: cached.success_rate,
            uptime_secs: cached.uptime_secs,
            recent_error_count: cached.recent_error_count,
            recent_warning_count: cached.recent_warning_count,
            reliability_score: cached.reliability_score,
        };

        let print_history = PrintHistorySummary {
            total_prints: cached.total_prints,
            successful_prints: cached.successful_prints,
            failed_prints: cached.failed_prints,
            success_rate: cached.success_rate,
            avg_duration_secs: cached.avg_duration_secs,
            recent_failures: cached.recent_failures.clone(),
            common_failure_pattern: most_common_failure_pattern(&cached.recent_failures),
        };

        let summary = PrinterSummary {
            name: cached.name.clone(),
            model: cached.model.clone(),
            firmware: cached.firmware.clone(),
            first_seen: cached.first_seen,
            last_seen: cached.last_seen,
            reliability_score: cached.reliability_score,
            total_observations: cached.total_observations,
            total_prints: cached.total_prints,
        };

        let current_state = CurrentState {
            is_printing: cached.is_printing,
            active_print_filename: cached.active_print_filename.clone(),
            active_observations: cached.active_observations.clone(),
            pending_warnings: cached.pending_warnings.clone(),
            recent_events: cached
                .evidence
                .iter()
                .rev()
                .take(MAX_RECENT_EVENTS)
                .cloned()
                .collect(),
        };

        Some(PrinterContext {
            printer_id: printer_id.into(),
            generated_at: Utc::now(),
            summary,
            print_history,
            health,
            current_state,
            known_issues: cached.known_issues.clone(),
            historical_patterns: cached.patterns.clone(),
            recent_evidence: cached
                .evidence
                .iter()
                .rev()
                .take(MAX_EVIDENCE)
                .cloned()
                .collect(),
        })
    }

    /// Return the number of printers with cached context.
    pub fn printer_count(&self) -> usize {
        self.printers.len()
    }

    fn process(&mut self, knowledge: Knowledge) {
        let cached = self
            .printers
            .entry(knowledge.printer_id.clone())
            .or_insert_with(|| CachedContext::new(knowledge.printer_id.clone()));

        let ts = knowledge.timestamp;

        match knowledge.kind {
            KnowledgeKind::ObservationTracked {
                observation_id,
                importance,
                confidence,
            } => {
                cached.total_observations += 1;
                cached.evidence.push(Evidence::inferred(
                    "observation_tracked",
                    &format!(
                        "Observation recorded (importance={:.2}, confidence={:.2})",
                        importance, confidence
                    ),
                    confidence,
                    ts,
                ));
                if cached.evidence.len() > MAX_EVIDENCE {
                    cached.evidence.remove(0);
                }
                let _ = observation_id;
            }

            KnowledgeKind::ObservationResolved {
                observation_id,
                resolution,
            } => {
                cached.evidence.push(Evidence {
                    fact_type: "observation_resolved".into(),
                    statement: format!("Observation resolved: {}", resolution),
                    quality: EvidenceQuality::Confirmed,
                    confidence: 1.0,
                    timestamp: ts,
                    source_id: Some(observation_id),
                });
                if cached.evidence.len() > MAX_EVIDENCE {
                    cached.evidence.remove(0);
                }
            }

            KnowledgeKind::ProfileUpdated { profile } => {
                cached.name = profile.printer_id.clone();
                cached.model = profile.hardware.model.clone();
                cached.firmware = profile.hardware.firmware.clone();
                cached.successful_prints = profile.behavior.successful_prints;
                cached.failed_prints = profile.behavior.failed_prints;
                cached.avg_duration_secs = profile.behavior.avg_print_duration_secs;

                // Compute success rate.
                let total = cached.successful_prints + cached.failed_prints;
                cached.total_prints = total;
                cached.success_rate = if total > 0 {
                    Some(cached.successful_prints as f64 / total as f64)
                } else {
                    None
                };
                cached.reliability_score = profile.behavior.reliability_score;

                // Known issues.
                cached.known_issues = profile
                    .behavior
                    .known_issues
                    .iter()
                    .map(|i| IssueSummary {
                        category: format!("{:?}", i.category),
                        description: i.description.clone(),
                        first_seen: i.first_seen,
                        last_seen: i.last_seen,
                        occurrence_count: i.occurrence_count,
                        resolved: i.resolved,
                        importance: importance_from_occurrence(i.occurrence_count),
                    })
                    .collect();

                // Build historical patterns from known issues.
                cached.patterns = profile
                    .behavior
                    .known_issues
                    .iter()
                    .filter(|i| i.occurrence_count > 1)
                    .map(|i| HistoricalPattern {
                        pattern_type: format!("{:?}", i.category),
                        description: i.description.clone(),
                        occurrence_count: i.occurrence_count,
                        first_seen: i.first_seen,
                        last_seen: i.last_seen,
                        typical_severity: "warning".into(),
                        resolved_count: if i.resolved { 1 } else { 0 },
                    })
                    .collect();

                // Active observations from unresolved issues.
                cached.active_observations = profile
                    .behavior
                    .known_issues
                    .iter()
                    .filter(|i| !i.resolved)
                    .map(|i| ObservationSummary {
                        category: format!("{:?}", i.category),
                        severity: "warning".into(),
                        message: i.description.clone(),
                        importance: importance_from_occurrence(i.occurrence_count),
                        confidence: 0.7,
                        quality: EvidenceQuality::Inferred,
                        timestamp: i.last_seen,
                    })
                    .collect();

                // Pending warnings from unresolved issues.
                cached.pending_warnings = profile
                    .behavior
                    .known_issues
                    .iter()
                    .filter(|i| !i.resolved)
                    .map(|i| i.description.clone())
                    .collect();

                // Track last seen.
                cached.last_seen = Some(profile.updated_at);
                if cached.first_seen.is_none() {
                    cached.first_seen = Some(profile.updated_at);
                }
            }

            KnowledgeKind::TimelineEventAdded { entry } => {
                // Track print lifecycle.
                match entry.event_type {
                    layermind_shared::knowledge::TimelineEventType::FailureDetected => {
                        cached.recent_failures.push(RecentFailure {
                            timestamp: entry.occurred_at,
                            reason: Some(entry.description.clone()),
                            failure_count_in_window: 1,
                        });
                        if cached.recent_failures.len() > MAX_RECENT_FAILURES {
                            cached.recent_failures.remove(0);
                        }
                        cached.evidence.push(Evidence::observed(
                            "print_failure",
                            &entry.description,
                            0.95,
                            entry.occurred_at,
                        ));
                    }
                    layermind_shared::knowledge::TimelineEventType::IssueResolved => {
                        cached.evidence.push(Evidence {
                            fact_type: "issue_resolved".into(),
                            statement: entry.description,
                            quality: EvidenceQuality::Confirmed,
                            confidence: 0.9,
                            timestamp: entry.occurred_at,
                            source_id: None,
                        });
                    }
                    _ => {}
                }
            }

            KnowledgeKind::KnowledgeSnapshot {
                active_observation_count,
                resolved_observation_count,
                timeline_event_count,
                ..
            } => {
                let _ = (
                    active_observation_count,
                    resolved_observation_count,
                    timeline_event_count,
                );
            }

            other => {
                tracing::debug!(kind = ?other, "unhandled knowledge kind in context engine");
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn importance_from_occurrence(count: u64) -> f64 {
    (0.3 + (count as f64).ln() * 0.15).clamp(0.0, 1.0)
}

fn most_common_failure_pattern(failures: &[RecentFailure]) -> Option<String> {
    if failures.is_empty() {
        return None;
    }

    let mut counts: HashMap<&str, u64> = HashMap::new();
    for f in failures {
        if let Some(ref reason) = f.reason {
            *counts.entry(reason.as_str()).or_default() += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(reason, _)| reason.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::knowledge::{
        KnownIssue, PrinterBehavior, PrinterHardware, PrinterProfile,
    };
    use layermind_shared::observation::AnomalyCategory;
    use uuid::Uuid;

    fn profile_updated(printer_id: &str) -> Knowledge {
        let mut profile = PrinterProfile::new(printer_id.into());
        profile.hardware.model = Some("Ender 3 V2".into());
        profile.hardware.firmware = Some("Marlin 2.1".into());
        profile.behavior.successful_prints = 42;
        profile.behavior.failed_prints = 3;
        profile.behavior.reliability_score = Some(0.85);
        profile.behavior.known_issues.push(KnownIssue {
            category: AnomalyCategory::TemperatureInstability,
            description: "PID oscillation on extruder".into(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            occurrence_count: 3,
            resolved: false,
        });
        Knowledge::new(printer_id.into(), KnowledgeKind::ProfileUpdated { profile })
    }

    #[test]
    fn empty_context_returns_none() {
        let (_, rx) = broadcast::channel(1);
        let engine = ContextEngine::new(rx);
        assert!(engine.context("nonexistent").is_none());
    }

    #[test]
    fn profile_update_populates_context() {
        let (tx, rx) = broadcast::channel(1);
        let mut engine = ContextEngine::new(rx);

        // Feed a knowledge record manually.
        engine.process(profile_updated("printer-1"));

        let ctx = engine.context("printer-1").unwrap();
        assert_eq!(ctx.printer_id, "printer-1");
        assert_eq!(ctx.summary.model.as_deref(), Some("Ender 3 V2"));
        assert_eq!(ctx.print_history.successful_prints, 42);
        assert_eq!(ctx.print_history.failed_prints, 3);
        assert!(ctx.print_history.success_rate.unwrap() > 0.9);
        assert_eq!(ctx.known_issues.len(), 1);
        assert_eq!(ctx.known_issues[0].occurrence_count, 3);
        assert!(!ctx.known_issues[0].resolved);
        assert_eq!(ctx.historical_patterns.len(), 1);
    }

    #[test]
    fn resolved_issue_clears_active_state() {
        let (tx, rx) = broadcast::channel(1);
        let mut engine = ContextEngine::new(rx);

        // First: profile with unresolved issue.
        engine.process(profile_updated("printer-1"));

        // Then: profile with resolved issue.
        let mut profile = PrinterProfile::new("printer-1".into());
        profile.behavior.known_issues.push(KnownIssue {
            category: AnomalyCategory::TemperatureInstability,
            description: "PID oscillation".into(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            occurrence_count: 3,
            resolved: true,
        });
        engine.process(Knowledge::new(
            "printer-1".into(),
            KnowledgeKind::ProfileUpdated { profile },
        ));

        let ctx = engine.context("printer-1").unwrap();
        assert!(ctx.known_issues[0].resolved);
        assert!(ctx.current_state.active_observations.is_empty());
        assert!(ctx.current_state.pending_warnings.is_empty());
    }

    #[test]
    fn health_summary_includes_reliability() {
        let (tx, rx) = broadcast::channel(1);
        let mut engine = ContextEngine::new(rx);

        engine.process(profile_updated("printer-1"));

        let ctx = engine.context("printer-1").unwrap();
        assert_eq!(ctx.health.reliability_score, Some(0.85));
    }
}
