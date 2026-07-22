//! Response parser — extracts structured Recommendation from raw AI output.
//!
//! The AI is instructed to return JSON, but may produce malformed output,
//! markdown-wrapped JSON, or JSON with missing fields. This parser
//! handles all cases gracefully — it never panics on bad AI output.

use layermind_shared::observation::Severity;
use layermind_shared::recommendation::{Action, Recommendation, RecommendationCategory, Reference};
use tracing;

/// Parsed recommendation with validation status.
#[derive(Debug, Clone)]
pub struct ParsedRecommendation {
    pub recommendation: Recommendation,
    /// Fields that were missing from the AI response and filled with defaults.
    pub missing_fields: Vec<String>,
    /// Whether the parser had to recover from malformed input.
    pub recovered_from_error: bool,
}

/// Attempt to parse a structured Recommendation from raw AI output.
///
/// Handles:
/// - Valid JSON — direct deserialization
/// - Markdown-wrapped JSON (```json ... ```) — strips fences
/// - Malformed JSON — returns a minimal fallback
/// - Missing fields — fills with safe defaults, records what was missing
pub fn parse_recommendation(printer_id: &str, raw_content: &str) -> ParsedRecommendation {
    let (json_str, recovered) = extract_json(raw_content);

    match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(value) => build_from_json(printer_id, &value, recovered),
        Err(e) => {
            tracing::warn!(error = %e, raw = %raw_content, "failed to parse AI response");
            fallback_recommendation(printer_id, raw_content, true)
        }
    }
}

/// Extract a JSON string from potentially malformed AI output.
fn extract_json(raw: &str) -> (String, bool) {
    let trimmed = raw.trim();

    // Try extracting from markdown code fence.
    if let Some(inner) = extract_markdown_json(trimmed) {
        return (inner, false);
    }

    // Try the raw string as-is.
    if trimmed.starts_with('{') {
        return (trimmed.to_string(), false);
    }

    // Look for the first { and last } and try that.
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            return (trimmed[start..=end].to_string(), true);
        }
    }

    // Give up — return the raw content for fallback.
    (raw.to_string(), true)
}

fn extract_markdown_json(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 3 {
        return None;
    }

    let start = lines.first()?.trim();
    let end = lines.last()?.trim();

    if (start == "```json" || start == "```") && end == "```" {
        let inner: String = lines[1..lines.len() - 1].join("\n");
        return Some(inner);
    }

    None
}

fn build_from_json(
    printer_id: &str,
    value: &serde_json::Value,
    recovered: bool,
) -> ParsedRecommendation {
    let mut missing = Vec::new();

    let category = value
        .get("category")
        .and_then(|v| v.as_str())
        .map(parse_category)
        .unwrap_or_else(|| {
            missing.push("category".into());
            RecommendationCategory::General
        });

    let severity = value
        .get("severity")
        .and_then(|v| v.as_str())
        .map(parse_severity)
        .unwrap_or_else(|| {
            missing.push("severity".into());
            Severity::Info
        });

    let confidence = value
        .get("confidence")
        .and_then(|v| v.as_f64())
        .map(|c| c.clamp(0.0, 1.0))
        .unwrap_or_else(|| {
            missing.push("confidence".into());
            0.5
        });

    let summary = value
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("No diagnosis available")
        .to_string();

    let explanation = value
        .get("explanation")
        .and_then(|v| v.as_str())
        .unwrap_or("The AI did not provide an explanation.")
        .to_string();

    let actions: Vec<Action> = value
        .get("actions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(Action {
                        priority: a.get("priority").and_then(|v| v.as_u64()).unwrap_or(99) as u8,
                        description: a
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unspecified action")
                            .into(),
                        suggested_command: a
                            .get("suggested_command")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty() && *s != "null")
                            .map(String::from),
                        expected_outcome: a
                            .get("expected_outcome")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Outcome not specified")
                            .into(),
                        is_safe_automatic: a
                            .get("is_safe_automatic")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    if actions.is_empty() && category != RecommendationCategory::General {
        missing.push("actions (empty)".into());
    }

    let evidence: Vec<Reference> = value
        .get("evidence")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| {
                    Some(Reference {
                        claim: e
                            .get("claim")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unspecified claim")
                            .into(),
                        supporting_fact: e
                            .get("supporting_fact")
                            .and_then(|v| v.as_str())
                            .unwrap_or("No evidence cited")
                            .into(),
                        source_quality: layermind_shared::context::EvidenceQuality::Inferred,
                        source_id: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut recommendation =
        Recommendation::new(printer_id.into(), category, severity, confidence, summary);
    recommendation.explanation = explanation;
    recommendation.actions = actions;
    recommendation.evidence = evidence;

    ParsedRecommendation {
        recommendation,
        missing_fields: missing,
        recovered_from_error: recovered,
    }
}

fn fallback_recommendation(printer_id: &str, raw: &str, recovered: bool) -> ParsedRecommendation {
    let summary = if raw.len() > 200 {
        format!("AI response could not be parsed ({} chars)", raw.len())
    } else {
        format!("AI response could not be parsed: {}", raw)
    };

    ParsedRecommendation {
        recommendation: Recommendation::new(
            printer_id.into(),
            RecommendationCategory::General,
            Severity::Info,
            0.0,
            summary,
        ),
        missing_fields: vec!["entire response".into()],
        recovered_from_error: recovered,
    }
}

fn parse_category(s: &str) -> RecommendationCategory {
    match s.to_lowercase().as_str() {
        "thermal" => RecommendationCategory::Thermal,
        "mechanical" => RecommendationCategory::Mechanical,
        "calibration" => RecommendationCategory::Calibration,
        "filament" => RecommendationCategory::Filament,
        "firmware" => RecommendationCategory::Firmware,
        _ => RecommendationCategory::General,
    }
}

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "warning" => Severity::Warning,
        _ => Severity::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json() {
        let json = r#"{
            "category": "thermal",
            "severity": "warning",
            "confidence": 0.85,
            "summary": "Temperature instability detected",
            "explanation": "The extruder shows 3°C oscillation.",
            "actions": [
                {
                    "priority": 1,
                    "description": "Run PID calibration",
                    "suggested_command": "PID_CALIBRATE HEATER=extruder TARGET=210",
                    "expected_outcome": "Stable temperatures",
                    "is_safe_automatic": false
                }
            ],
            "evidence": [
                {
                    "claim": "Temperature is unstable",
                    "supporting_fact": "Average deviation 3.2°C over 20 readings"
                }
            ]
        }"#;

        let result = parse_recommendation("printer-1", json);
        assert!(!result.recovered_from_error);
        assert!(result.missing_fields.is_empty());
        assert_eq!(
            result.recommendation.category,
            RecommendationCategory::Thermal
        );
        assert_eq!(result.recommendation.severity, Severity::Warning);
        assert_eq!(result.recommendation.actions.len(), 1);
        assert_eq!(result.recommendation.evidence.len(), 1);
        assert!(result.recommendation.actions[0].suggested_command.is_some());
    }

    #[test]
    fn parse_markdown_wrapped_json() {
        let raw = "```json\n{\"category\":\"general\",\"severity\":\"info\",\"confidence\":1.0,\"summary\":\"All good\",\"explanation\":\"\",\"actions\":[],\"evidence\":[]}\n```";
        let result = parse_recommendation("p1", raw);
        assert!(!result.recovered_from_error);
        assert_eq!(
            result.recommendation.category,
            RecommendationCategory::General
        );
    }

    #[test]
    fn parse_missing_fields_fills_defaults() {
        let json = r#"{"summary": "bare minimum"}"#;
        let result = parse_recommendation("p1", json);
        assert!(!result.missing_fields.is_empty());
        assert_eq!(
            result.recommendation.category,
            RecommendationCategory::General
        );
        assert_eq!(result.recommendation.severity, Severity::Info);
        assert!(result.recommendation.actions.is_empty());
    }

    #[test]
    fn parse_malformed_json_returns_fallback() {
        let raw = "not json at all, just some text";
        let result = parse_recommendation("p1", raw);
        assert!(result.recovered_from_error);
        assert_eq!(result.recommendation.severity, Severity::Info);
        assert_eq!(
            result.recommendation.category,
            RecommendationCategory::General
        );
    }

    #[test]
    fn parse_recommendation_detects_recovery() {
        let raw = "Here's my diagnosis: {\"category\":\"mechanical\",\"severity\":\"warning\",\"confidence\":0.7,\"summary\":\"Belt tension\",\"explanation\":\"\",\"actions\":[],\"evidence\":[]} end";
        let result = parse_recommendation("p1", raw);
        assert!(result.recovered_from_error);
        assert_eq!(
            result.recommendation.category,
            RecommendationCategory::Mechanical
        );
    }

    #[test]
    fn null_suggested_command_not_included() {
        let json = r#"{"category":"general","severity":"info","confidence":1.0,"summary":"ok","explanation":"","actions":[{"priority":1,"description":"test","suggested_command":null,"expected_outcome":"ok","is_safe_automatic":true}],"evidence":[]}"#;
        let result = parse_recommendation("p1", json);
        assert!(result.recommendation.actions[0].suggested_command.is_none());
    }

    #[test]
    fn empty_string_suggested_command_not_included() {
        let json = r#"{"category":"general","severity":"info","confidence":1.0,"summary":"ok","explanation":"","actions":[{"priority":1,"description":"test","suggested_command":"","expected_outcome":"ok","is_safe_automatic":true}],"evidence":[]}"#;
        let result = parse_recommendation("p1", json);
        assert!(result.recommendation.actions[0].suggested_command.is_none());
    }
}
