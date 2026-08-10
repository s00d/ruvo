//! Uniform JSON snapshot helpers (insta).

/// Assert JSON snapshot with common redactions (`id`, timestamps).
///
/// Uses `$crate::insta` so callers of this crate (or the facade re-export) do not
/// need a direct `insta` dependency.
#[macro_export]
macro_rules! assert_json_snapshot {
    ($name:expr, $value:expr) => {{
        let mut settings = $crate::insta::Settings::clone_current();
        $crate::with_json_redactions(&mut settings);
        settings.bind(|| {
            $crate::insta::assert_json_snapshot!($name, $value);
        });
    }};
    ($value:expr) => {{
        let mut settings = $crate::insta::Settings::clone_current();
        $crate::with_json_redactions(&mut settings);
        settings.bind(|| {
            $crate::insta::assert_json_snapshot!($value);
        });
    }};
}

/// Redact volatile fields commonly present in API JSON.
pub fn with_json_redactions(settings: &mut insta::Settings) {
    settings.add_filter(r#""id"\s*:\s*\d+"#, r#""id": "[id]""#);
    settings.add_filter(
        r#""created_at"\s*:\s*"[^"]*""#,
        r#""created_at": "[datetime]""#,
    );
    settings.add_filter(
        r#""updated_at"\s*:\s*"[^"]*""#,
        r#""updated_at": "[datetime]""#,
    );
    settings.add_filter(
        r#""read_at"\s*:\s*("[^"]*"|null)"#,
        r#""read_at": "[read_at]""#,
    );
}
