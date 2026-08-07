//! Resolve the Cargo package and optional `[frontend]` from `ruvo.toml`.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Project {
    /// Directory that contains the package `Cargo.toml`.
    pub package_dir: PathBuf,
    /// Workspace / project root (where `cargo` should run).
    pub workspace_dir: PathBuf,
    pub package_name: String,
    /// Binary name for `target/release/<bin>`.
    pub bin_name: String,
    pub frontend: Option<FrontendConfig>,
}

#[derive(Debug, Clone)]
pub struct FrontendConfig {
    pub dir: PathBuf,
    pub dev: String,
    pub build: String,
    pub out: PathBuf,
    pub package_manager: String,
}

#[derive(Debug, Default, Deserialize)]
struct RuvoTomlFrontend {
    enabled: Option<bool>,
    dir: Option<String>,
    dev: Option<String>,
    build: Option<String>,
    out: Option<String>,
    package_manager: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RuvoToml {
    frontend: Option<RuvoTomlFrontend>,
}

/// Common options for `dev` / `build` / `serve`.
#[derive(Debug, Clone)]
pub struct ProjectOpts {
    pub package: Option<String>,
    pub manifest_path: Option<PathBuf>,
}

impl Project {
    pub fn resolve(opts: &ProjectOpts) -> Result<Self, String> {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let start = opts
            .manifest_path
            .as_ref()
            .map(|p| {
                if p.is_file() {
                    p.parent().unwrap_or(Path::new(".")).to_path_buf()
                } else {
                    p.clone()
                }
            })
            .unwrap_or(cwd);

        let package_manifest = find_package_manifest(&start, opts.package.as_deref())?;
        let package_dir = package_manifest
            .parent()
            .ok_or_else(|| "invalid Cargo.toml path".to_string())?
            .to_path_buf();
        let workspace_dir = find_workspace_root(&package_dir).unwrap_or_else(|| package_dir.clone());

        let manifest = fs::read_to_string(&package_manifest).map_err(|e| e.to_string())?;
        let package_name = parse_package_name(&manifest)?;
        let bin_name = parse_bin_name(&manifest).unwrap_or_else(|| package_name.clone());

        if let Some(ref want) = opts.package {
            if want != &package_name {
                return Err(format!(
                    "resolved package `{package_name}` but `-p {want}` was requested"
                ));
            }
        }

        let frontend = resolve_frontend(&package_dir, &workspace_dir)?;

        Ok(Self {
            package_dir,
            workspace_dir,
            package_name,
            bin_name,
            frontend,
        })
    }

    pub fn release_bin(&self) -> PathBuf {
        let p = self
            .workspace_dir
            .join("target")
            .join("release")
            .join(&self.bin_name);
        #[cfg(windows)]
        {
            let mut p = p;
            p.set_extension("exe");
            return p;
        }
        #[cfg(not(windows))]
        {
            p
        }
    }
}

fn find_package_manifest(start: &Path, package: Option<&str>) -> Result<PathBuf, String> {
    if let Some(name) = package {
        let mut dir = start.to_path_buf();
        loop {
            let manifest = dir.join("Cargo.toml");
            if manifest.is_file() {
                let text = fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
                if text.contains("[workspace]") {
                    return find_package_in_workspace(&dir, name);
                }
                if parse_package_name(&text).ok().as_deref() == Some(name) {
                    return Ok(manifest);
                }
            }
            if !dir.pop() {
                break;
            }
        }
        // Also try workspace from cwd ancestors even if we didn't hit [workspace] with matching name
        if let Some(ws) = find_workspace_root(start) {
            return find_package_in_workspace(&ws, name);
        }
        return Err(format!(
            "package `{name}` not found from {}",
            start.display()
        ));
    }

    let mut dir = start.to_path_buf();
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let text = fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
            if text.contains("[package]") {
                return Ok(manifest);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    Err(format!(
        "no Cargo.toml with [package] found from {}",
        start.display()
    ))
}

fn find_workspace_root(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(text) = fs::read_to_string(&manifest) {
                if text.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn find_package_in_workspace(workspace: &Path, name: &str) -> Result<PathBuf, String> {
    for root in ["examples", "crates", "plugins", "."] {
        let root = if root == "." {
            workspace.to_path_buf()
        } else {
            workspace.join(root)
        };
        if !root.is_dir() {
            continue;
        }
        if let Ok(found) = walk_for_package(&root, name, 4) {
            return Ok(found);
        }
    }
    Err(format!(
        "package `{name}` not found under {}",
        workspace.display()
    ))
}

fn walk_for_package(dir: &Path, name: &str, depth: usize) -> Result<PathBuf, String> {
    if depth == 0 {
        return Err("not found".into());
    }
    let manifest = dir.join("Cargo.toml");
    if manifest.is_file() {
        if let Ok(text) = fs::read_to_string(&manifest) {
            if parse_package_name(&text).ok().as_deref() == Some(name) {
                return Ok(manifest);
            }
        }
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for ent in entries.flatten() {
        let path = ent.path();
        if path.is_dir() {
            let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if fname == "target" || fname == "node_modules" || fname.starts_with('.') {
                continue;
            }
            if let Ok(found) = walk_for_package(&path, name, depth - 1) {
                return Ok(found);
            }
        }
    }
    Err("not found".into())
}

fn parse_package_name(manifest: &str) -> Result<String, String> {
    let value: toml::Value = toml::from_str(manifest).map_err(|e| e.to_string())?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Cargo.toml missing [package].name".into())
}

fn parse_bin_name(manifest: &str) -> Option<String> {
    let value: toml::Value = toml::from_str(manifest).ok()?;
    if let Some(bins) = value.get("bin").and_then(|b| b.as_array()) {
        if let Some(first) = bins.first() {
            if let Some(name) = first.get("name").and_then(|n| n.as_str()) {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn load_ruvo_toml(dirs: &[&Path]) -> Option<RuvoToml> {
    for dir in dirs {
        for name in ["ruvo.toml", "Ruvo.toml"] {
            let path = dir.join(name);
            if path.is_file() {
                if let Ok(text) = fs::read_to_string(&path) {
                    if let Ok(doc) = toml::from_str::<RuvoToml>(&text) {
                        return Some(doc);
                    }
                }
            }
        }
    }
    None
}

fn detect_package_manager(dir: &Path) -> String {
    if dir.join("pnpm-lock.yaml").is_file() {
        "pnpm".into()
    } else if dir.join("yarn.lock").is_file() {
        "yarn".into()
    } else if dir.join("bun.lockb").is_file() || dir.join("bun.lock").is_file() {
        "bun".into()
    } else {
        "npm".into()
    }
}

fn package_json_looks_like_vite(dir: &Path) -> bool {
    let path = dir.join("package.json");
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    text.contains("\"vite\"") || text.contains("'vite'")
}

fn resolve_frontend(
    package_dir: &Path,
    workspace_dir: &Path,
) -> Result<Option<FrontendConfig>, String> {
    let ruvo = load_ruvo_toml(&[package_dir, workspace_dir]);
    let section = ruvo.and_then(|r| r.frontend);

    if let Some(ref s) = section {
        if s.enabled == Some(false) {
            return Ok(None);
        }
    }

    let configured_dir = section.as_ref().and_then(|s| s.dir.as_ref()).map(|d| {
        let p = PathBuf::from(d);
        if p.is_absolute() {
            p
        } else {
            package_dir.join(p)
        }
    });

    let auto_dir = if package_dir.join("frontend").join("package.json").is_file() {
        Some(package_dir.join("frontend"))
    } else if package_json_looks_like_vite(package_dir) {
        Some(package_dir.to_path_buf())
    } else {
        None
    };

    let dir = configured_dir.or(auto_dir);

    let enabled = match section.as_ref().and_then(|s| s.enabled) {
        Some(true) => true,
        Some(false) => return Ok(None),
        None => dir.is_some(),
    };

    if !enabled {
        return Ok(None);
    }

    let dir = dir.ok_or_else(|| {
        "[frontend] enabled but no dir / package.json found (set frontend.dir)".to_string()
    })?;

    if !dir.join("package.json").is_file() {
        return Err(format!(
            "frontend dir {} has no package.json",
            dir.display()
        ));
    }

    let pm = section
        .as_ref()
        .and_then(|s| s.package_manager.clone())
        .unwrap_or_else(|| detect_package_manager(&dir));

    let run = |script: &str| match pm.as_str() {
        "pnpm" => format!("pnpm run {script}"),
        "yarn" => format!("yarn {script}"),
        "bun" => format!("bun run {script}"),
        _ => format!("npm run {script}"),
    };

    let dev = section
        .as_ref()
        .and_then(|s| s.dev.clone())
        .unwrap_or_else(|| run("dev"));
    let build = section
        .as_ref()
        .and_then(|s| s.build.clone())
        .unwrap_or_else(|| run("build"));

    let out = section
        .as_ref()
        .and_then(|s| s.out.as_ref())
        .map(|o| {
            let p = PathBuf::from(o);
            if p.is_absolute() {
                p
            } else {
                package_dir.join(p)
            }
        })
        .unwrap_or_else(|| package_dir.join("public").join("build"));

    Ok(Some(FrontendConfig {
        dir,
        dev,
        build,
        out,
        package_manager: pm,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_name() {
        let m = r#"
[package]
name = "cabinet"
version = "0.1.0"
"#;
        assert_eq!(parse_package_name(m).unwrap(), "cabinet");
    }
}
