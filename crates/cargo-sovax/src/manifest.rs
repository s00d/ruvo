//! Patch consumer `Cargo.toml` / `src/main.rs` for generated stacks.

use std::fs;
use std::path::Path;

use crate::util::io_err;

/// Ensure `sova` feature list includes each of `need`.
pub fn ensure_sova_features(need: &[&str]) -> Result<(), String> {
    let path = Path::new("Cargo.toml");
    let raw = fs::read_to_string(path).map_err(io_err)?;
    let mut doc: toml::Value = raw.parse().map_err(|e| format!("Cargo.toml: {e}"))?;
    let deps = doc
        .get_mut("dependencies")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| "Cargo.toml missing [dependencies]".to_string())?;

    let sova = deps
        .get_mut("sova")
        .ok_or_else(|| "Cargo.toml missing sova dependency".to_string())?;

    let features = match sova {
        toml::Value::Table(t) => t
            .entry("features")
            .or_insert_with(|| toml::Value::Array(vec![])),
        _ => return Err("sova dependency must be a table".into()),
    };
    let arr = features
        .as_array_mut()
        .ok_or_else(|| "sova.features must be an array".to_string())?;
    for f in need {
        let already = arr.iter().any(|v| v.as_str() == Some(f));
        if !already {
            arr.push(toml::Value::String((*f).to_string()));
        }
    }

    write_toml(path, &doc)
}

/// Insert a dependency if missing (value is a TOML fragment for the right-hand side).
pub fn ensure_dep(name: &str, rhs: &str) -> Result<(), String> {
    let path = Path::new("Cargo.toml");
    let raw = fs::read_to_string(path).map_err(io_err)?;
    if raw.contains(&format!("{name} =")) || raw.contains(&format!("{name}={{")) {
        return Ok(());
    }
    let mut doc: toml::Value = raw.parse().map_err(|e| format!("Cargo.toml: {e}"))?;
    let deps = doc
        .get_mut("dependencies")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| "Cargo.toml missing [dependencies]".to_string())?;
    let parsed: toml::Value = format!("{name} = {rhs}")
        .parse()
        .map_err(|e| format!("bad dep `{name}`: {e}"))?;
    let table = parsed
        .as_table()
        .ok_or_else(|| "internal: expected table".to_string())?;
    let value = table
        .get(name)
        .cloned()
        .ok_or_else(|| "internal: missing key".to_string())?;
    deps.insert(name.to_string(), value);
    write_toml(path, &doc)
}

fn write_toml(path: &Path, doc: &toml::Value) -> Result<(), String> {
    let out = toml::to_string_pretty(doc).map_err(|e| format!("serialize Cargo.toml: {e}"))?;
    fs::write(path, out).map_err(io_err)?;
    Ok(())
}

/// Features + crates needed for `generate resource` / model / migration / seed.
pub fn ensure_resource_stack(api: bool) -> Result<(), String> {
    let mut feats = vec!["db-sqlite", "vld", "env"];
    if api {
        // `api` preset already has openapi+vld; still ensure db.
        feats.push("openapi");
        feats.push("vld-openapi");
    } else {
        feats.push("vld-form");
        feats.push("vld-flash");
    }
    ensure_sova_features(&feats)?;
    ensure_dep("serde", r#"{ version = "1", features = ["derive"] }"#)?;
    ensure_dep("serde_json", r#""1""#)?;
    ensure_dep(
        "sea-orm",
        r#"{ version = "2.0", default-features = false, features = ["runtime-tokio-rustls", "macros", "sqlx-sqlite"] }"#,
    )?;
    ensure_dep(
        "sea-orm-migration",
        r#"{ version = "2.0", default-features = false, features = ["runtime-tokio-rustls", "sqlx-sqlite"] }"#,
    )?;
    ensure_dep("async-trait", r#""0.1""#)?;
    ensure_db_in_main()?;
    Ok(())
}

pub fn ensure_mail_stack() -> Result<(), String> {
    ensure_sova_features(&["mail-templates"])?;
    ensure_dep("serde_json", r#""1""#)?;
    Ok(())
}

pub fn ensure_tasks_stack() -> Result<(), String> {
    ensure_sova_features(&["tasks"])?;
    Ok(())
}

/// Wire `mod entities` / `mod migrations` and `Db::from_env().migrations::<…>()`.
pub fn ensure_db_in_main() -> Result<(), String> {
    let path = Path::new("src/main.rs");
    if !path.exists() {
        return Ok(());
    }
    let mut src = fs::read_to_string(path).map_err(io_err)?;

    if !src.contains("mod entities") {
        if let Some(idx) = src.find("mod modules;") {
            src.insert_str(idx, "mod entities;\nmod migrations;\n");
        } else if let Some(idx) = src.find("#[tokio::main]") {
            src.insert_str(idx, "mod entities;\nmod migrations;\n\n");
        } else {
            return Err("could not find insertion point for mod entities in src/main.rs".into());
        }
    }

    if !src.contains("use sova::Db") && !src.contains("Db::from_env") {
        // Prefer extending an existing `use sova::{…}` if present.
        if let Some(start) = src.find("use sova::{") {
            let rest = &src[start..];
            if let Some(end_rel) = rest.find("};") {
                let end = start + end_rel;
                let block = &src[start..=end + 1];
                if !block.contains("Db") {
                    src.insert_str(end, ", Db");
                }
            }
        } else if let Some(idx) = src.find("use sova::prelude::*;") {
            let insert_at = idx + "use sova::prelude::*;\n".len();
            src.insert_str(insert_at, "use sova::Db;\n");
        } else {
            src.insert_str(0, "use sova::Db;\n");
        }
    }

    if !src.contains("Db::from_env()") {
        // After App::web()/api() builder chain, before modules::register / app.run.
        if let Some(idx) = src.find("modules::register") {
            src.insert_str(
                idx,
                "app.install(Db::from_env().migrations::<migrations::Migrator>());\n    ",
            );
        } else if let Some(idx) = src.find("app.run()") {
            src.insert_str(
                idx,
                "app.install(Db::from_env().migrations::<migrations::Migrator>());\n    ",
            );
        } else {
            return Err("could not find place to install Db in src/main.rs".into());
        }
    }

    fs::write(path, src).map_err(io_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static CWD: Mutex<()> = Mutex::new(());

    #[test]
    fn merges_sova_features_and_deps() {
        let _g = CWD.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let prev = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        fs::write(
            "Cargo.toml",
            r#"[package]
name = "t"
version = "0.1.0"
edition = "2021"

[dependencies]
sova = { version = "0.1", features = ["web"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
"#,
        )
        .unwrap();
        fs::create_dir_all("src").unwrap();
        fs::write(
            "src/main.rs",
            r#"use sova::prelude::*;
use sova::{Html, Meta, Parser, ServerArgs};

mod modules;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();
    let mut app = App::web().site("t").public_url("http://127.0.0.1:3000");
    modules::register(&mut app);
    app.run().await
}
"#,
        )
        .unwrap();

        ensure_resource_stack(false).unwrap();
        let toml = fs::read_to_string("Cargo.toml").unwrap();
        assert!(toml.contains("db-sqlite"), "{toml}");
        assert!(toml.contains("vld-form"), "{toml}");
        assert!(toml.contains("sea-orm"), "{toml}");
        assert!(toml.contains("serde_json"), "{toml}");

        let main = fs::read_to_string("src/main.rs").unwrap();
        assert!(main.contains("mod entities;"), "{main}");
        assert!(main.contains("mod migrations;"), "{main}");
        assert!(
            main.contains("Db::from_env().migrations::<migrations::Migrator>()"),
            "{main}"
        );

        env::set_current_dir(prev).unwrap();
    }
}
