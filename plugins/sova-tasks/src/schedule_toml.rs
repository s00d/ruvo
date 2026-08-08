//! Parse `[schedule.<job>]` from [`sova_core::ConfigDoc`].

use crate::job::{parse_cron, Schedule};
use sova_core::extend::parse_duration;
use sova_core::ConfigDoc;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct TomlSchedule {
    pub name: String,
    pub schedule: Schedule,
    pub queue: Option<String>,
    pub priority: Option<i32>,
    pub payload: Value,
}

/// Entries under `[schedule]` / `[schedule.name]` (nested tables keyed by job name).
pub(crate) fn parse_schedule_toml(
    doc: &ConfigDoc,
) -> Result<Vec<TomlSchedule>, String> {
    let Some(section) = doc.section("schedule") else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (name, val) in section {
        let toml::Value::Table(t) = val else {
            return Err(format!(
                "schedule.{name}: expected a table with cron= or every="
            ));
        };
        let cron = t.get("cron").and_then(|v| v.as_str());
        let every = t.get("every").and_then(|v| v.as_str());
        let schedule = match (cron, every) {
            (Some(expr), None) => Schedule::Cron(
                parse_cron(expr).map_err(|e| format!("schedule.{name} cron: {e}"))?,
            ),
            (None, Some(s)) => {
                let d = parse_duration(s)
                    .map_err(|e| format!("schedule.{name} every: {e}"))?;
                if d.is_zero() {
                    return Err(format!("schedule.{name} every must be non-zero"));
                }
                Schedule::Every(d)
            }
            (Some(_), Some(_)) => {
                return Err(format!(
                    "schedule.{name}: set either cron= or every=, not both"
                ));
            }
            (None, None) => {
                return Err(format!(
                    "schedule.{name}: missing cron= or every="
                ));
            }
        };
        let queue = t
            .get("queue")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let priority = t.get("priority").and_then(|v| {
            v.as_integer()
                .map(|i| i as i32)
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        });
        let payload = t
            .get("payload")
            .map(toml_to_json)
            .unwrap_or_else(|| Value::Object(Default::default()));
        out.push(TomlSchedule {
            name,
            schedule,
            queue,
            priority,
            payload,
        });
    }
    Ok(out)
}

fn toml_to_json(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut m = serde_json::Map::new();
            for (k, v) in t {
                m.insert(k.clone(), toml_to_json(v));
            }
            Value::Object(m)
        }
    }
}

/// Human label for list/schedule CLI.
pub(crate) fn schedule_label(s: &Schedule) -> String {
    match s {
        Schedule::Every(d) => format!("every {}", format_duration(*d)),
        Schedule::Cron(c) => format!("cron {c}"),
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 && secs.is_multiple_of(3600) {
        format!("{}h", secs / 3600)
    } else if secs >= 60 && secs.is_multiple_of(60) {
        format!("{}m", secs / 60)
    } else if d.subsec_millis() == 0 {
        format!("{secs}s")
    } else {
        format!("{}ms", d.as_millis())
    }
}

/// Merge code schedules with toml (toml wins per job name).
pub(crate) fn merge_schedules(
    mut by_name: HashMap<String, crate::schedule::ScheduledJob>,
    toml_entries: Vec<TomlSchedule>,
    known_handlers: &HashMap<String, crate::Handler>,
    default_queue: &str,
    job_queues: &HashMap<String, String>,
    job_priorities: &HashMap<String, i32>,
) -> Result<Vec<crate::schedule::ScheduledJob>, Vec<String>> {
    let mut unknown = Vec::new();
    for e in toml_entries {
        if !known_handlers.contains_key(&e.name) {
            unknown.push(e.name.clone());
            continue;
        }
        let queue = e
            .queue
            .or_else(|| job_queues.get(&e.name).cloned())
            .unwrap_or_else(|| default_queue.to_string());
        let priority = e
            .priority
            .or_else(|| job_priorities.get(&e.name).copied())
            .unwrap_or(0);
        by_name.insert(
            e.name.clone(),
            crate::schedule::ScheduledJob {
                name: e.name,
                schedule: e.schedule,
                payload: e.payload,
                queue,
                priority,
            },
        );
    }
    if !unknown.is_empty() {
        return Err(unknown);
    }
    Ok(by_name.into_values().collect())
}
