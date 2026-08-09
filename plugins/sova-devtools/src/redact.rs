//! Mask secrets in session dumps / config.

#[cfg_attr(not(feature = "session"), allow(dead_code))]
pub fn mask_value(key: &str, value: &str) -> String {
    let k = key.to_ascii_lowercase();
    let sensitive = k.contains("password")
        || k.contains("secret")
        || k.contains("token")
        || k.contains("api_key")
        || k.contains("apikey")
        || k.contains("authorization")
        || k.contains("cookie")
        || k.ends_with("_key")
        || k.contains("passwd");
    if sensitive {
        return "***".into();
    }
    if value.len() > 120 {
        format!("{}…", &value[..117])
    } else {
        value.to_string()
    }
}

pub fn redact_sql_bindings(sql: &str) -> String {
    // Keep SQL text; truncate huge statements.
    if sql.len() > 2000 {
        format!("{}…", &sql[..1997])
    } else {
        sql.to_string()
    }
}
