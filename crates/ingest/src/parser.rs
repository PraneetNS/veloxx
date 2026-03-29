//! Log format parser.
//!
//! Handles three common log formats:
//! - **JSON** — structured JSON objects.
//! - **logfmt** — `key=value key="quoted value"` pairs.
//! - **Plain text with level prefix** — e.g. `[ERROR] something broke`.

use std::collections::HashMap;
use common::telemetry::{LogLevel, LogPayload};
use once_cell::sync::Lazy;
use regex::Regex;

/// Regex matching common log level prefixes at the start of a line:
/// `[ERROR]`, `ERROR:`, `<ERROR>`, bare `ERROR `, etc.
static LEVEL_PREFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^\s*[\[\(<]?(TRACE|TRC|DEBUG|DBG|INFO|INF|INFORMATION|WARN|WARNING|WRN|ERROR|ERR|ERRO|FATAL|CRITICAL|CRIT|PANIC)[\]>):\s-]*\s*",
    )
    .expect("invalid level prefix regex")
});

/// Parse a raw log string into a [`LogPayload`].
///
/// Tries JSON first, then logfmt, then falls back to plain-text with
/// optional level prefix detection.
pub fn parse_log(raw: &str) -> LogPayload {
    let raw = raw.trim();

    // 1. Try JSON
    if raw.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
            return from_json(val);
        }
    }

    // 2. Try logfmt
    if raw.contains('=') {
        let pairs = parse_logfmt(raw);
        if !pairs.is_empty() {
            return from_logfmt(pairs, raw);
        }
    }

    // 3. Plain text with optional level prefix
    from_plain(raw)
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

fn from_json(val: serde_json::Value) -> LogPayload {
    let obj = match val.as_object() {
        Some(o) => o,
        None    => return plain_payload("INFO", val.to_string()),
    };

    // Try common message keys
    let message = ["message", "msg", "log", "text", "body"]
        .iter()
        .find_map(|k| obj.get(*k)?.as_str().map(str::to_owned))
        .unwrap_or_else(|| val.to_string());

    // Try common level keys
    let level_str = ["level", "severity", "lvl", "loglevel"]
        .iter()
        .find_map(|k| obj.get(*k)?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "INFO".to_owned());

    let trace_id  = obj.get("trace_id").or_else(|| obj.get("traceId"))
        .and_then(|v| v.as_str()).map(str::to_owned);
    let span_id   = obj.get("span_id").or_else(|| obj.get("spanId"))
        .and_then(|v| v.as_str()).map(str::to_owned);

    // Collect remaining keys as extra fields
    let skip = ["message", "msg", "log", "text", "body", "level", "severity",
                "lvl", "loglevel", "trace_id", "traceId", "span_id", "spanId"];
    let fields: HashMap<String, serde_json::Value> = obj
        .iter()
        .filter(|(k, _)| !skip.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    LogPayload {
        level: LogLevel::parse(&level_str),
        message,
        trace_id,
        span_id,
        fields,
    }
}

// ---------------------------------------------------------------------------
// logfmt
// ---------------------------------------------------------------------------

/// Parse a logfmt string into a `HashMap<String, String>`.
fn parse_logfmt(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut rest = s;

    while !rest.is_empty() {
        rest = rest.trim_start();
        // Find `=`
        let Some(eq) = rest.find('=') else { break };
        let key = rest[..eq].trim().to_owned();
        rest = &rest[eq + 1..];

        if rest.starts_with('"') {
            // Quoted value
            rest = &rest[1..];
            let end = rest.find('"').unwrap_or(rest.len());
            let value = rest[..end].to_owned();
            rest = if end < rest.len() { &rest[end + 1..] } else { "" };
            map.insert(key, value);
        } else {
            let end = rest.find(' ').unwrap_or(rest.len());
            let value = rest[..end].to_owned();
            rest = &rest[end..];
            map.insert(key, value);
        }
    }

    map
}

fn from_logfmt(mut pairs: HashMap<String, String>, raw: &str) -> LogPayload {
    let message = pairs
        .remove("message")
        .or_else(|| pairs.remove("msg"))
        .or_else(|| pairs.remove("log"))
        .unwrap_or_else(|| raw.to_owned());

    let level_str = pairs
        .remove("level")
        .or_else(|| pairs.remove("severity"))
        .unwrap_or_else(|| "INFO".to_owned());

    let trace_id = pairs.remove("trace_id").or_else(|| pairs.remove("traceId"));
    let span_id  = pairs.remove("span_id").or_else(|| pairs.remove("spanId"));

    let fields = pairs
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();

    LogPayload {
        level: LogLevel::parse(&level_str),
        message,
        trace_id,
        span_id,
        fields,
    }
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

fn from_plain(raw: &str) -> LogPayload {
    if let Some(caps) = LEVEL_PREFIX_RE.captures(raw) {
        let level_str = caps.get(1).map_or("INFO", |m| m.as_str());
        let level   = LogLevel::parse(level_str);
        let message = raw[caps.get(0).unwrap().end()..].to_owned();
        return LogPayload {
            level,
            message,
            trace_id: None,
            span_id:  None,
            fields:   HashMap::new(),
        };
    }
    plain_payload("INFO", raw.to_owned())
}

fn plain_payload(level: &str, message: String) -> LogPayload {
    LogPayload {
        level:    LogLevel::parse(level),
        message,
        trace_id: None,
        span_id:  None,
        fields:   HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Prometheus metric parsing
// ---------------------------------------------------------------------------

/// A parsed Prometheus metric sample.
#[derive(Debug, Clone)]
pub struct PromSample {
    pub name:   String,
    pub labels: HashMap<String, String>,
    pub value:  f64,
}

/// Parse a Prometheus text-format exposition payload.
///
/// Skips comment (`#`) and blank lines.  Each sample line has the form:
/// `metric_name{label="val",...} <value> [timestamp]`
pub fn parse_prometheus(body: &str) -> Vec<PromSample> {
    let mut samples = Vec::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(sample) = parse_prom_line(line) {
            samples.push(sample);
        }
    }

    samples
}

fn parse_prom_line(line: &str) -> Option<PromSample> {
    // Split off optional timestamp (third whitespace-separated token).
    let mut parts = line.splitn(3, ' ');
    let name_and_labels = parts.next()?;
    let value_str       = parts.next()?;

    let value: f64 = value_str.parse().ok()?;

    let (name, labels) = if let Some(brace) = name_and_labels.find('{') {
        let name   = name_and_labels[..brace].to_owned();
        let lpart  = &name_and_labels[brace + 1..name_and_labels.len().saturating_sub(1)];
        let labels = parse_prom_labels(lpart);
        (name, labels)
    } else {
        (name_and_labels.to_owned(), HashMap::new())
    };

    Some(PromSample { name, labels, value })
}

fn parse_prom_labels(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in s.split(',') {
        let pair = pair.trim();
        if let Some(eq) = pair.find('=') {
            let k = pair[..eq].trim().to_owned();
            let v = pair[eq + 1..].trim().trim_matches('"').to_owned();
            map.insert(k, v);
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_log() {
        let raw = r#"{"level":"error","message":"db timeout","trace_id":"abc123"}"#;
        let p = parse_log(raw);
        assert_eq!(p.level, LogLevel::Error);
        assert_eq!(p.message, "db timeout");
        assert_eq!(p.trace_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_logfmt_log() {
        let raw = r#"level=warn msg="high latency" service=api duration=450ms"#;
        let p = parse_log(raw);
        assert_eq!(p.level, LogLevel::Warn);
        assert_eq!(p.message, "high latency");
    }

    #[test]
    fn parse_plain_log_with_prefix() {
        let raw = "[ERROR] something exploded";
        let p = parse_log(raw);
        assert_eq!(p.level, LogLevel::Error);
        assert_eq!(p.message, "something exploded");
    }

    #[test]
    fn loose_level_parsing() {
        assert_eq!(LogLevel::parse("ERR"),     LogLevel::Error);
        assert_eq!(LogLevel::parse("WARNING"), LogLevel::Warn);
        assert_eq!(LogLevel::parse("CRIT"),    LogLevel::Fatal);
    }

    #[test]
    fn parse_prom_line_basic() {
        let body = "# HELP foo A counter\nfoo{job=\"bar\"} 42.0\n";
        let samples = parse_prometheus(body);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].name, "foo");
        assert_eq!(samples[0].value, 42.0);
        assert_eq!(samples[0].labels.get("job").map(String::as_str), Some("bar"));
    }
}
