//! AI recommendation types — the output of the Reasoning Engine.
//!
//! Recommendations are structured diagnostic outputs produced by AI
//! models. Every recommendation carries an evidence trail linking
//! claims back to observed/inferred facts, a trust assessment, and
//! usage tracking for cost monitoring.
//!
//! Feedback types are included as a foundation for a future learning
//! loop where user outcomes improve printer knowledge over time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::EvidenceQuality;
use crate::observation::Severity;

// ── Recommendation ──────────────────────────────────────────────────

/// A structured AI diagnostic for a printer issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: Uuid,
    pub printer_id: String,
    pub category: RecommendationCategory,
    pub severity: Severity,
    /// AI model's own confidence in this recommendation (0.0–1.0).
    pub confidence: f64,
    /// One-line diagnosis.
    pub summary: String,
    /// Narrative reasoning — what the AI thinks is happening and why.
    pub explanation: String,
    /// Ordered action plan (priority 1 = do this first).
    pub actions: Vec<Action>,
    /// Citations anchoring AI claims to specific evidence in the context.
    pub evidence: Vec<Reference>,
    /// Resource usage for this recommendation generation.
    pub usage: AiUsage,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationCategory {
    Mechanical,
    Thermal,
    Calibration,
    Filament,
    Firmware,
    General,
}

// ── Action ───────────────────────────────────────────────────────────

/// A single action item the AI recommends. Advisory only — the user
/// must approve before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Execution order (1 = do this first).
    pub priority: u8,
    /// Human-readable description of what to do.
    pub description: String,
    /// An optional G-code command the user might run. NOT auto-executed.
    pub suggested_command: Option<String>,
    /// What the AI expects to happen if this action is taken.
    pub expected_outcome: String,
    /// Whether this action is safe to execute without human review.
    pub is_safe_automatic: bool,
}

// ── Reference / Evidence Citation ────────────────────────────────────

/// Anchors an AI claim to a specific piece of evidence from the
/// PrinterContext that was provided to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// The claim the AI made.
    pub claim: String,
    /// The evidence fact that supports this claim.
    pub supporting_fact: String,
    /// How we know the supporting fact.
    pub source_quality: EvidenceQuality,
    /// Optional backlink to the originating observation or knowledge record.
    pub source_id: Option<Uuid>,
}

// ── Trust & Validation ───────────────────────────────────────────────

/// The output after trust validation — what the user actually sees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedRecommendation {
    #[serde(flatten)]
    pub recommendation: Recommendation,
    pub trust: TrustAssessment,
    pub disclaimers: Vec<String>,
    /// Per-action explainability chain — why each action was recommended.
    pub explanation_factors: Vec<ExplanationFactor>,
    /// Contradictions detected between evidence sources during diagnosis.
    pub contradictions: Vec<Contradiction>,
}

/// Mechanical cross-reference of AI claims against context evidence.
/// Deterministic — does not call the AI model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAssessment {
    pub overall_trust: TrustLevel,
    /// Claims anchored to Observed evidence (sensor data).
    pub facts_cited: usize,
    /// Claims anchored to Inferred evidence (rule output).
    pub inferences_made: usize,
    /// Claims with no matching evidence in the context.
    pub unsupported_claims: usize,
    /// 0.0–1.0, derived from unsupported_claims / total_claims.
    pub hallucination_risk: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    High,
    Medium,
    Low,
}

// ── AI Usage Tracking ────────────────────────────────────────────────

/// Token usage and estimated cost for one AI request.
/// Foundation for future billing / subscription models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiUsage {
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub estimated_cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

// ── User Feedback (future learning loop) ─────────────────────────────

/// Outcome of a recommendation as reported by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackResult {
    Fixed,
    NotFixed,
    Incorrect,
    Helpful,
}

/// User feedback on a recommendation. Foundation for future learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub recommendation_id: Uuid,
    pub result: FeedbackResult,
    pub notes: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ── Explainability ──────────────────────────────────────────────────

/// Why a particular action was recommended — the reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationFactor {
    /// The reason this factor contributes (e.g. "temperature_instability").
    pub reason: String,
    /// Indices into the recommendation's evidence array.
    pub evidence_refs: Vec<usize>,
    /// What we assumed that may not be directly observed.
    pub assumption: Option<String>,
    /// Whether this factor is directly observed, inferred, or confirmed.
    pub observation_type: EvidenceQuality,
    /// How strongly this factor supports the action (0.0–1.0).
    pub weight: f64,
}

// ── Historical Trends ──────────────────────────────────────────────

/// How an issue or pattern has evolved over time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trend {
    /// First time this has been observed.
    New,
    /// Same issue has appeared before (count >= 2).
    Recurring,
    /// Frequency or severity is increasing.
    Worsening,
    /// Frequency or severity is decreasing.
    Improving,
    /// No significant change.
    Unchanged,
    /// Previously active, now resolved.
    RecentlyResolved,
}

// ── Contradictions ─────────────────────────────────────────────────

/// A contradiction detected between two pieces of evidence in the context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub description: String,
    /// What the first source claims.
    pub item_a: String,
    /// What the second source claims (conflicting).
    pub item_b: String,
    pub severity: ContradictionSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionSeverity {
    Minor,
    Significant,
    Critical,
}

// ── Constructors ─────────────────────────────────────────────────────

impl Recommendation {
    pub fn new(
        printer_id: String,
        category: RecommendationCategory,
        severity: Severity,
        confidence: f64,
        summary: String,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            printer_id,
            category,
            severity,
            confidence,
            summary,
            explanation: String::new(),
            actions: Vec::new(),
            evidence: Vec::new(),
            usage: AiUsage::default(),
            created_at: Utc::now(),
        }
    }
}

impl Default for AiUsage {
    fn default() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
            prompt_tokens: 0,
            completion_tokens: 0,
            estimated_cost_usd: 0.0,
            timestamp: Utc::now(),
        }
    }
}

impl AiUsage {
    pub fn new(provider: &str, model: &str, prompt_tokens: u32, completion_tokens: u32) -> Self {
        let cost = estimate_cost(model, prompt_tokens, completion_tokens);
        Self {
            provider: provider.into(),
            model: model.into(),
            prompt_tokens,
            completion_tokens,
            estimated_cost_usd: cost,
            timestamp: Utc::now(),
        }
    }
}

impl ValidatedRecommendation {
    pub fn new(recommendation: Recommendation, trust: TrustAssessment) -> Self {
        let mut disclaimers = Vec::new();

        if trust.overall_trust == TrustLevel::Low {
            disclaimers
                .push("Low trust: significant unsupported claims. Verify before acting.".into());
        }
        if trust.hallucination_risk > 0.3 {
            disclaimers
                .push("Hallucination risk elevated. Cross-check against printer state.".into());
        }
        if trust.unsupported_claims > 0 {
            disclaimers.push(format!(
                "{} claim(s) could not be verified against known evidence.",
                trust.unsupported_claims
            ));
        }

        Self {
            recommendation,
            trust,
            disclaimers,
            explanation_factors: Vec::new(),
            contradictions: Vec::new(),
        }
    }
}

impl Feedback {
    pub fn new(recommendation_id: Uuid, result: FeedbackResult, notes: Option<String>) -> Self {
        Self {
            recommendation_id,
            result,
            notes,
            timestamp: Utc::now(),
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Rough cost estimation for known models. Falls back to
/// $0.002/1K tokens for unknown models.
fn estimate_cost(model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
    let (prompt_price_per_1k, completion_price_per_1k): (f64, f64) = match model {
        m if m.contains("gpt-4o") => (0.0025, 0.01),
        m if m.contains("gpt-4") => (0.03, 0.06),
        m if m.contains("gpt-3.5") => (0.0005, 0.0015),
        m if m.contains("claude-3-opus") => (0.015, 0.075),
        m if m.contains("claude-3-sonnet") => (0.003, 0.015),
        m if m.contains("claude-3-haiku") => (0.00025, 0.00125),
        m if m.contains("deepseek") => (0.00014, 0.00028),
        m if m.contains("mixtral") || m.contains("mistral") => (0.0002, 0.0002),
        _ => (0.002, 0.002), // conservative default
    };

    (prompt_tokens as f64 / 1000.0) * prompt_price_per_1k
        + (completion_tokens as f64 / 1000.0) * completion_price_per_1k
}
