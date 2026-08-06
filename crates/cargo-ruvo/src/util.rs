use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn validate_ident(raw: &str) -> Result<(), String> {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return Err("name must not be empty".into());
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!("invalid name `{raw}`"));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("invalid name `{raw}`"));
    }
    Ok(())
}

pub fn sanitize_crate_name(raw: &str) -> Result<String, String> {
    let base = Path::new(raw)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(raw);
    let cleaned: String = base
        .chars()
        .map(|c| if c == '-' || c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let cleaned = cleaned.trim_matches('_').to_string();
    let cleaned = cleaned.trim_start_matches(|c: char| c.is_ascii_digit()).to_string();
    if cleaned.is_empty() {
        return Err(format!("invalid package name derived from `{raw}`"));
    }
    if !cleaned.chars().next().is_some_and(|c| c == '_' || c.is_ascii_alphabetic()) {
        return Err(format!("invalid package name `{cleaned}` from `{raw}`"));
    }
    Ok(cleaned)
}

pub fn to_type_name(raw: &str) -> String {
    raw.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut out = s.chars();
            let first = out.next().unwrap().to_ascii_uppercase();
            let mut built = String::new();
            built.push(first);
            built.extend(out);
            built
        })
        .collect::<Vec<_>>()
        .join("")
}

/// UTC `YYYYMMDD_HHMMSS` without external deps.
pub fn utc_ymdhms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d, hh, mm, ss) = civil_from_unix(secs);
    format!("{y:04}{m:02}{d:02}_{hh:02}{mm:02}{ss:02}")
}

fn civil_from_unix(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let ss = (secs % 60) as u32;
    let mins = secs / 60;
    let mm = (mins % 60) as u32;
    let hours = mins / 60;
    let hh = (hours % 24) as u32;
    let mut days = (hours / 24) as i64;

    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as u32;
    (y, m, d, hh, mm, ss)
}

pub fn io_err(err: std::io::Error) -> String {
    err.to_string()
}

pub fn path_err(err: std::path::StripPrefixError) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ident_ok() {
        validate_ident("users").unwrap();
        validate_ident("_x").unwrap();
    }

    #[test]
    fn validate_ident_rejects() {
        assert!(validate_ident("").is_err());
        assert!(validate_ident("1bad").is_err());
        assert!(validate_ident("has-dash").is_err());
    }

    #[test]
    fn sanitize_and_type_name() {
        assert_eq!(sanitize_crate_name("my-app").unwrap(), "my-app");
        assert_eq!(to_type_name("blog_post"), "BlogPost");
    }
}

