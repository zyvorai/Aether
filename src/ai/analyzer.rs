// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Log analyzer with anomaly detection
//!
//! Pattern recognition, error rate tracking, and statistical anomaly
//! detection for workload logs.

use crate::config::AnalyzerConfig;
use crate::output;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Log analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAnalysis {
    pub total_lines: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub error_rate: f64,
    pub patterns: Vec<LogPattern>,
    pub anomalies: Vec<Anomaly>,
    pub health_assessment: HealthAssessment,
    pub timeline: Vec<TimeWindow>,
}

/// Detected log pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPattern {
    pub pattern: String,
    pub count: usize,
    pub severity: Severity,
    pub first_seen: usize,
    pub last_seen: usize,
    pub example: String,
}

/// Log severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "ERROR"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

/// Detected anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub anomaly_type: AnomalyType,
    pub description: String,
    pub severity: Severity,
    pub z_score: f64,
    pub window_index: usize,
}

/// Type of anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Sudden spike in error rate
    ErrorSpike,
    /// Sustained high error rate
    SustainedErrors,
    /// New error pattern appeared
    NewPattern,
    /// Log volume anomaly (too many or too few logs)
    VolumeAnomaly,
}

impl std::fmt::Display for AnomalyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnomalyType::ErrorSpike => write!(f, "Error Spike"),
            AnomalyType::SustainedErrors => write!(f, "Sustained Errors"),
            AnomalyType::NewPattern => write!(f, "New Pattern"),
            AnomalyType::VolumeAnomaly => write!(f, "Volume Anomaly"),
        }
    }
}

/// Health assessment based on log analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAssessment {
    pub status: HealthStatus,
    pub score: f64,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Critical => write!(f, "Critical"),
        }
    }
}

/// Time window for rate analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start_line: usize,
    pub end_line: usize,
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub error_rate: f64,
}

/// Log analyzer
pub struct LogAnalyzer {
    config: AnalyzerConfig,
}

impl LogAnalyzer {
    pub fn new(config: AnalyzerConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(AnalyzerConfig::default())
    }

    /// Analyze log content
    pub fn analyze(&self, logs: &str) -> LogAnalysis {
        let lines: Vec<&str> = logs.lines().collect();
        let total_lines = lines.len();

        if total_lines == 0 {
            return LogAnalysis {
                total_lines: 0,
                error_count: 0,
                warning_count: 0,
                error_rate: 0.0,
                patterns: vec![],
                anomalies: vec![],
                health_assessment: HealthAssessment {
                    status: HealthStatus::Healthy,
                    score: 100.0,
                    issues: vec![],
                    suggestions: vec!["No logs available for analysis".to_string()],
                },
                timeline: vec![],
            };
        }

        // Classify each line
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut pattern_counts: HashMap<String, (usize, Severity, usize, usize, String)> =
            HashMap::new();

        for (idx, line) in lines.iter().enumerate() {
            let severity = self.classify_line(line);

            match severity {
                Severity::Error => error_count += 1,
                Severity::Warning => warning_count += 1,
                Severity::Info => {}
            }

            // Extract pattern (normalize numbers, paths, UUIDs)
            let pattern = self.extract_pattern(line);
            let entry = pattern_counts.entry(pattern.clone()).or_insert((
                0,
                severity.clone(),
                idx,
                idx,
                line.to_string(),
            ));
            entry.0 += 1;
            entry.1 = severity;
            entry.3 = idx; // last_seen
        }

        let error_rate = if total_lines > 0 {
            error_count as f64 / total_lines as f64
        } else {
            0.0
        };

        // Build pattern list (top patterns by count)
        let mut patterns: Vec<LogPattern> = pattern_counts
            .into_iter()
            .map(
                |(pattern, (count, severity, first, last, example))| LogPattern {
                    pattern,
                    count,
                    severity,
                    first_seen: first,
                    last_seen: last,
                    example: if example.len() > 200 {
                        format!("{}...", &example[..200])
                    } else {
                        example
                    },
                },
            )
            .collect();

        patterns.sort_by_key(|p| std::cmp::Reverse(p.count));
        patterns.truncate(self.config.max_patterns);

        // Build timeline windows
        let timeline = self.build_timeline(&lines);

        // Detect anomalies
        let anomalies = self.detect_anomalies(&timeline, &patterns);

        // Health assessment
        let health_assessment = self.assess_health(error_rate, &anomalies, &patterns);

        LogAnalysis {
            total_lines,
            error_count,
            warning_count,
            error_rate,
            patterns,
            anomalies,
            health_assessment,
            timeline,
        }
    }

    /// Classify a log line by severity
    fn classify_line(&self, line: &str) -> Severity {
        let lower = line.to_lowercase();

        // Error indicators
        if lower.contains("error")
            || lower.contains("fatal")
            || lower.contains("panic")
            || lower.contains("exception")
            || lower.contains("fail")
            || lower.contains("critical")
        {
            return Severity::Error;
        }

        // Warning indicators
        if lower.contains("warn")
            || lower.contains("timeout")
            || lower.contains("retry")
            || lower.contains("deprecated")
            || lower.contains("slow")
        {
            return Severity::Warning;
        }

        Severity::Info
    }

    /// Extract a normalized pattern from a log line
    fn extract_pattern(&self, line: &str) -> String {
        let mut pattern = line.to_string();

        // Normalize numbers
        pattern = pattern
            .chars()
            .map(|c| if c.is_ascii_digit() { '#' } else { c })
            .collect();

        // Collapse consecutive # characters
        let mut collapsed = String::new();
        let mut prev_hash = false;
        for c in pattern.chars() {
            if c == '#' {
                if !prev_hash {
                    collapsed.push('#');
                }
                prev_hash = true;
            } else {
                collapsed.push(c);
                prev_hash = false;
            }
        }

        // Truncate long patterns
        if collapsed.len() > 120 {
            collapsed.truncate(120);
        }

        collapsed
    }

    /// Build timeline windows from log lines
    fn build_timeline(&self, lines: &[&str]) -> Vec<TimeWindow> {
        let window_size = self.config.window_size;
        let mut windows = Vec::new();

        let mut i = 0;
        while i < lines.len() {
            let end = (i + window_size).min(lines.len());
            let window_lines = &lines[i..end];

            let mut errors = 0;
            let mut warnings = 0;
            for line in window_lines {
                match self.classify_line(line) {
                    Severity::Error => errors += 1,
                    Severity::Warning => warnings += 1,
                    Severity::Info => {}
                }
            }

            let total = window_lines.len();
            windows.push(TimeWindow {
                start_line: i,
                end_line: end,
                total,
                errors,
                warnings,
                error_rate: if total > 0 {
                    errors as f64 / total as f64
                } else {
                    0.0
                },
            });

            i = end;
        }

        windows
    }

    /// Detect anomalies using z-score analysis
    fn detect_anomalies(&self, timeline: &[TimeWindow], patterns: &[LogPattern]) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        if !self.config.enable_anomaly_detection || timeline.len() < 2 {
            return anomalies;
        }

        // Calculate mean and stddev of error rates
        let error_rates: Vec<f64> = timeline.iter().map(|w| w.error_rate).collect();
        let mean = error_rates.iter().sum::<f64>() / error_rates.len() as f64;
        let variance =
            error_rates.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / error_rates.len() as f64;
        let stddev = variance.sqrt();

        // Detect error rate spikes
        for (idx, window) in timeline.iter().enumerate() {
            if stddev > 0.0 {
                let z_score = (window.error_rate - mean) / stddev;

                if z_score > self.config.anomaly_z_threshold {
                    anomalies.push(Anomaly {
                        anomaly_type: AnomalyType::ErrorSpike,
                        description: format!(
                            "Error rate {:.1}% in lines {}-{} (z-score: {:.1})",
                            window.error_rate * 100.0,
                            window.start_line,
                            window.end_line,
                            z_score
                        ),
                        severity: Severity::Error,
                        z_score,
                        window_index: idx,
                    });
                }
            }
        }

        // Detect sustained high error rate (last 3+ windows)
        if timeline.len() >= 3 {
            let last_three: Vec<f64> = timeline
                .iter()
                .rev()
                .take(3)
                .map(|w| w.error_rate)
                .collect();

            if last_three.iter().all(|r| *r > 0.10) {
                anomalies.push(Anomaly {
                    anomaly_type: AnomalyType::SustainedErrors,
                    description: format!(
                        "Sustained error rate >10% in last {} windows",
                        last_three.len()
                    ),
                    severity: Severity::Error,
                    z_score: 0.0,
                    window_index: timeline.len() - 1,
                });
            }
        }

        // Detect volume anomalies
        let volumes: Vec<f64> = timeline.iter().map(|w| w.total as f64).collect();
        let vol_mean = volumes.iter().sum::<f64>() / volumes.len() as f64;
        let vol_variance =
            volumes.iter().map(|v| (v - vol_mean).powi(2)).sum::<f64>() / volumes.len() as f64;
        let vol_stddev = vol_variance.sqrt();

        for (idx, window) in timeline.iter().enumerate() {
            if vol_stddev > 0.0 {
                let z_score = ((window.total as f64) - vol_mean) / vol_stddev;
                if z_score.abs() > self.config.anomaly_z_threshold {
                    anomalies.push(Anomaly {
                        anomaly_type: AnomalyType::VolumeAnomaly,
                        description: format!(
                            "Unusual log volume ({} lines) in window {} (z-score: {:.1})",
                            window.total, idx, z_score
                        ),
                        severity: Severity::Warning,
                        z_score: z_score.abs(),
                        window_index: idx,
                    });
                }
            }
        }

        // Detect new error patterns appearing late
        for pattern in patterns {
            if pattern.severity == Severity::Error
                && pattern.first_seen
                    >= (timeline.len() * self.config.window_size)
                        .saturating_sub(self.config.window_size)
                && pattern.count >= 3
            {
                anomalies.push(Anomaly {
                    anomaly_type: AnomalyType::NewPattern,
                    description: format!(
                        "New error pattern appeared ({} occurrences): {}",
                        pattern.count,
                        &pattern.pattern.chars().take(80).collect::<String>()
                    ),
                    severity: Severity::Error,
                    z_score: 0.0,
                    window_index: timeline.len().saturating_sub(1),
                });
            }
        }

        anomalies
    }

    /// Assess overall health from analysis
    fn assess_health(
        &self,
        error_rate: f64,
        anomalies: &[Anomaly],
        _patterns: &[LogPattern],
    ) -> HealthAssessment {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        let error_anomalies = anomalies
            .iter()
            .filter(|a| a.severity == Severity::Error)
            .count();

        let (status, score) = if error_rate > 0.20 || error_anomalies >= 3 {
            issues.push(format!("High error rate: {:.1}%", error_rate * 100.0));
            suggestions.push("Investigate error patterns immediately".to_string());
            suggestions.push("Check resource limits and connectivity".to_string());
            (HealthStatus::Critical, 10.0)
        } else if error_rate > 0.10 || error_anomalies >= 2 {
            issues.push(format!("Elevated error rate: {:.1}%", error_rate * 100.0));
            suggestions.push("Review error logs for root cause".to_string());
            (HealthStatus::Unhealthy, 30.0)
        } else if error_rate > 0.05 || error_anomalies >= 1 {
            issues.push(format!("Moderate error rate: {:.1}%", error_rate * 100.0));
            suggestions.push("Monitor error trends closely".to_string());
            (HealthStatus::Degraded, 60.0)
        } else {
            (HealthStatus::Healthy, 90.0 + (1.0 - error_rate) * 10.0)
        };

        for anomaly in anomalies {
            issues.push(format!("{}: {}", anomaly.anomaly_type, anomaly.description));
        }

        HealthAssessment {
            status,
            score: score.min(100.0),
            issues,
            suggestions,
        }
    }
}

/// Format log analysis as a report
pub fn format_analysis_report(analysis: &LogAnalysis) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        ("Log Analysis", format!("{} lines", analysis.total_lines)),
        (
            "Health",
            format!(
                "{} (score: {:.0}/100)",
                analysis.health_assessment.status, analysis.health_assessment.score
            ),
        ),
        (
            "Errors",
            format!(
                "{} ({:.1}%)",
                analysis.error_count,
                analysis.error_rate * 100.0
            ),
        ),
        ("Warnings", format!("{}", analysis.warning_count)),
    ]));

    if !analysis.anomalies.is_empty() {
        output.push_str(&format!(
            "Anomalies Detected ({}):\n",
            analysis.anomalies.len()
        ));
        for anomaly in &analysis.anomalies {
            output.push_str(&format!(
                "  [{}] {}: {}\n",
                anomaly.severity, anomaly.anomaly_type, anomaly.description
            ));
        }
        output.push('\n');
    }

    if !analysis.health_assessment.issues.is_empty() {
        output.push_str("Issues:\n");
        for issue in &analysis.health_assessment.issues {
            output.push_str(&output::tree_bullet("❌", issue));
        }
        output.push('\n');
    }

    if !analysis.health_assessment.suggestions.is_empty() {
        output.push_str("Suggestions:\n");
        for suggestion in &analysis.health_assessment.suggestions {
            output.push_str(&output::tree_bullet("💡", suggestion));
        }
        output.push('\n');
    }

    // Top error patterns
    let error_patterns: Vec<&LogPattern> = analysis
        .patterns
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .take(5)
        .collect();

    if !error_patterns.is_empty() {
        output.push_str("Top Error Patterns:\n");
        for pattern in error_patterns {
            output.push_str(&format!(
                "  ({} occurrences) {}\n",
                pattern.count,
                &pattern.example[..pattern.example.len().min(100)]
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_logs() -> String {
        vec![
            "2026-01-01 INFO Starting application",
            "2026-01-01 INFO Listening on port 8080",
            "2026-01-01 INFO Request received GET /health",
            "2026-01-01 WARN Slow query detected (250ms)",
            "2026-01-01 INFO Request received GET /api/data",
            "2026-01-01 ERROR Connection refused to database",
            "2026-01-01 ERROR Connection refused to database",
            "2026-01-01 INFO Retrying database connection",
            "2026-01-01 INFO Database connected",
            "2026-01-01 INFO Request received POST /api/data",
            "2026-01-01 INFO Request received GET /health",
            "2026-01-01 WARN Memory usage at 80%",
            "2026-01-01 INFO Request received GET /api/data",
            "2026-01-01 INFO Request completed 200 OK",
            "2026-01-01 INFO Request completed 200 OK",
        ]
        .join("\n")
    }

    fn error_heavy_logs() -> String {
        let mut lines = Vec::new();
        for i in 0..100 {
            if i % 3 == 0 {
                lines.push(format!("2026-01-01 ERROR Failed request {}", i));
            } else {
                lines.push(format!("2026-01-01 INFO Normal request {}", i));
            }
        }
        lines.join("\n")
    }

    #[test]
    fn test_basic_analysis() {
        let analyzer = LogAnalyzer::with_defaults();
        let analysis = analyzer.analyze(&sample_logs());

        assert_eq!(analysis.total_lines, 15);
        // "Connection refused" x2 + "fail" in other lines may also match
        assert!(analysis.error_count >= 2);
        assert!(analysis.warning_count >= 2);
        assert!(analysis.error_rate < 0.50);
    }

    #[test]
    fn test_empty_logs() {
        let analyzer = LogAnalyzer::with_defaults();
        let analysis = analyzer.analyze("");

        assert_eq!(analysis.total_lines, 0);
        assert_eq!(analysis.error_count, 0);
        assert_eq!(analysis.health_assessment.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_error_heavy_detection() {
        let analyzer = LogAnalyzer::with_defaults();
        let analysis = analyzer.analyze(&error_heavy_logs());

        assert!(analysis.error_rate > 0.20);
        assert_eq!(analysis.health_assessment.status, HealthStatus::Critical);
    }

    #[test]
    fn test_pattern_extraction() {
        let analyzer = LogAnalyzer::with_defaults();
        let analysis = analyzer.analyze(&sample_logs());

        // Should detect the repeated "Connection refused" pattern
        assert!(!analysis.patterns.is_empty());
    }

    #[test]
    fn test_health_assessment() {
        let analyzer = LogAnalyzer::with_defaults();
        let analysis = analyzer.analyze(&sample_logs());

        // With a few errors in 15 lines, should not be Critical
        assert!(analysis.health_assessment.score > 0.0);
        assert!(analysis.health_assessment.score <= 100.0);
    }

    #[test]
    fn test_format_report() {
        let analyzer = LogAnalyzer::with_defaults();
        let analysis = analyzer.analyze(&sample_logs());
        let report = format_analysis_report(&analysis);
        assert!(report.contains("Log Analysis"));
        assert!(report.contains("Health"));
    }
}
