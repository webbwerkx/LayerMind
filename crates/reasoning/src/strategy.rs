//! Diagnostic strategy — configures how PrintDoctor runs the diagnostic
//! pipeline. Each strategy controls evidence selection, historical depth,
//! contradiction reporting, prompt verbosity, and explanation detail.
//!
//! Three built-in presets:
//!   - Rapid: minimal evidence, fast response, lower token usage.
//!   - Standard: Phase 2.4 behavior (the default).
//!   - Thorough: maximum evidence, deeper analysis, higher token usage.
//!
//! All strategies are deterministic — they configure the pipeline, not
//! the AI model. PrintDoctor always executes exactly one AI request.

/// Controls the behaviour of the diagnostic pipeline.
#[derive(Debug, Clone)]
pub struct DiagnosticStrategy {
    /// Human-readable name.
    pub name: &'static str,
    /// Maximum evidence items to include in the prompt.
    pub max_evidence: usize,
    /// Maximum known issues to include in the prompt.
    pub max_issues: usize,
    /// Maximum active observations to include.
    pub max_observations: usize,
    /// Maximum recent print failures to show.
    pub max_recent_failures: usize,
    /// Whether to include contradictions in the prompt.
    pub include_contradictions: bool,
    /// Whether to annotate issues with trend labels.
    pub include_historical_trends: bool,
    /// How verbose the prompt sections should be.
    pub prompt_verbosity: PromptVerbosity,
    /// How much explanation detail to generate per action.
    pub explanation_detail: ExplanationDetail,
    /// Tokens requested from the AI model.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptVerbosity {
    /// Short labels, minimal description.
    Abbreviated,
    /// Full sections with context (Phase 2.4 default).
    Standard,
    /// All available detail, extra context.
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationDetail {
    /// One-line per action, no evidence links.
    Minimal,
    /// Reason + evidence refs (Phase 2.4 default).
    Standard,
    /// Full factor breakdown with assumptions and weights.
    Full,
}

impl DiagnosticStrategy {
    /// Fast diagnosis — minimal evidence, lower tokens, faster response.
    pub const RAPID: Self = Self {
        name: "rapid",
        max_evidence: 5,
        max_issues: 3,
        max_observations: 5,
        max_recent_failures: 1,
        include_contradictions: false,
        include_historical_trends: false,
        prompt_verbosity: PromptVerbosity::Abbreviated,
        explanation_detail: ExplanationDetail::Minimal,
        max_tokens: 512,
        temperature: 0.5,
    };

    /// Balanced diagnosis — Phase 2.4 default behaviour.
    pub const STANDARD: Self = Self {
        name: "standard",
        max_evidence: 15,
        max_issues: 10,
        max_observations: 10,
        max_recent_failures: 3,
        include_contradictions: true,
        include_historical_trends: true,
        prompt_verbosity: PromptVerbosity::Standard,
        explanation_detail: ExplanationDetail::Standard,
        max_tokens: 1024,
        temperature: 0.3,
    };

    /// Deep diagnosis — maximum context, lower temperature, higher tokens.
    pub const THOROUGH: Self = Self {
        name: "thorough",
        max_evidence: 25,
        max_issues: 15,
        max_observations: 15,
        max_recent_failures: 5,
        include_contradictions: true,
        include_historical_trends: true,
        prompt_verbosity: PromptVerbosity::Detailed,
        explanation_detail: ExplanationDetail::Full,
        max_tokens: 2048,
        temperature: 0.1,
    };
}

impl Default for DiagnosticStrategy {
    fn default() -> Self {
        Self::STANDARD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapid_is_smaller_than_standard() {
        assert!(DiagnosticStrategy::RAPID.max_evidence < DiagnosticStrategy::STANDARD.max_evidence);
        assert!(DiagnosticStrategy::RAPID.max_tokens < DiagnosticStrategy::STANDARD.max_tokens);
    }

    #[test]
    fn thorough_is_larger_than_standard() {
        assert!(
            DiagnosticStrategy::THOROUGH.max_evidence > DiagnosticStrategy::STANDARD.max_evidence
        );
        assert!(DiagnosticStrategy::THOROUGH.max_tokens > DiagnosticStrategy::STANDARD.max_tokens);
    }

    #[test]
    fn standard_is_default() {
        let default = DiagnosticStrategy::default();
        assert_eq!(
            default.max_evidence,
            DiagnosticStrategy::STANDARD.max_evidence
        );
    }

    #[test]
    fn strategy_is_clone() {
        let s = DiagnosticStrategy::RAPID;
        let _s2 = s.clone();
    }

    #[test]
    fn rapid_has_smallest_limits() {
        let r = DiagnosticStrategy::RAPID;
        let s = DiagnosticStrategy::STANDARD;
        let t = DiagnosticStrategy::THOROUGH;
        assert!(r.max_evidence < s.max_evidence);
        assert!(r.max_issues < s.max_issues);
        assert!(r.max_tokens < s.max_tokens);
        assert!(t.max_evidence > s.max_evidence);
        assert!(t.max_tokens > s.max_tokens);
    }

    #[test]
    fn rapid_excludes_contradictions_and_trends() {
        let r = DiagnosticStrategy::RAPID;
        assert!(!r.include_contradictions);
        assert!(!r.include_historical_trends);
    }

    #[test]
    fn standard_and_thorough_include_contradictions() {
        assert!(DiagnosticStrategy::STANDARD.include_contradictions);
        assert!(DiagnosticStrategy::THOROUGH.include_contradictions);
    }

    #[test]
    fn thorough_has_lowest_temperature() {
        assert!(
            DiagnosticStrategy::THOROUGH.temperature < DiagnosticStrategy::STANDARD.temperature
        );
        assert!(DiagnosticStrategy::THOROUGH.temperature < DiagnosticStrategy::RAPID.temperature);
    }

    #[test]
    fn rapid_is_abbreviated_verbosity() {
        assert_eq!(
            DiagnosticStrategy::RAPID.prompt_verbosity,
            PromptVerbosity::Abbreviated
        );
        assert_eq!(
            DiagnosticStrategy::STANDARD.prompt_verbosity,
            PromptVerbosity::Standard
        );
        assert_eq!(
            DiagnosticStrategy::THOROUGH.prompt_verbosity,
            PromptVerbosity::Detailed
        );
    }

    #[test]
    fn custom_strategy_overrides_work() {
        let custom = DiagnosticStrategy {
            max_evidence: 7,
            max_tokens: 777,
            ..DiagnosticStrategy::STANDARD
        };
        assert_eq!(custom.max_evidence, 7);
        assert_eq!(custom.max_tokens, 777);
        // Inherited fields unchanged.
        assert_eq!(custom.max_issues, DiagnosticStrategy::STANDARD.max_issues);
        assert_eq!(custom.include_contradictions, true);
    }
}
