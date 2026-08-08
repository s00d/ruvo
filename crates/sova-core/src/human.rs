//! Human-readable sizes and durations (`"2 MiB"`, `"30s"`).

use std::time::Duration;

/// Parse `"2 MiB"`, `"512 KiB"`, `"1024"`, `"1MB"` → bytes.
pub fn parse_bytes(input: &str) -> Result<usize, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("empty size".into());
    }
    let (num_str, unit) = split_num_unit(s)?;
    let n: f64 = num_str
        .parse()
        .map_err(|_| format!("invalid size number: {num_str}"))?;
    if n < 0.0 {
        return Err("size must be non-negative".into());
    }
    let mult = match unit {
        "" | "b" | "B" => 1.0,
        "k" | "K" | "kb" | "KB" | "KiB" => 1024.0,
        "m" | "M" | "mb" | "MB" | "MiB" => 1024.0 * 1024.0,
        "g" | "G" | "gb" | "GB" | "GiB" => 1024.0 * 1024.0 * 1024.0,
        other => return Err(format!("unknown size unit: {other}")),
    };
    Ok((n * mult) as usize)
}

/// Parse `"30s"`, `"5m"`, `"1h"`, `"500ms"` → [`Duration`].
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("empty duration".into());
    }
    let (num_str, unit) = split_num_unit(s)?;
    let n: f64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration number: {num_str}"))?;
    if n < 0.0 {
        return Err("duration must be non-negative".into());
    }
    let secs = match unit {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => n,
        "ms" | "millis" | "millisecond" | "milliseconds" => n / 1000.0,
        "m" | "min" | "mins" | "minute" | "minutes" => n * 60.0,
        "h" | "hr" | "hour" | "hours" => n * 3600.0,
        other => return Err(format!("unknown duration unit: {other}")),
    };
    Ok(Duration::from_secs_f64(secs))
}

fn split_num_unit(s: &str) -> Result<(&str, &str), String> {
    let end = s
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 {
        return Err(format!("missing number in `{s}`"));
    }
    Ok((&s[..end], s[end..].trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_units() {
        assert_eq!(parse_bytes("1024").unwrap(), 1024);
        assert_eq!(parse_bytes("2 KiB").unwrap(), 2048);
        assert_eq!(parse_bytes("1MiB").unwrap(), 1024 * 1024);
        assert_eq!(parse_bytes("50 MiB").unwrap(), 50 * 1024 * 1024);
    }

    #[test]
    fn duration_units() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
    }
}
