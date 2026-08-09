//! Path jail: relative paths under a canonical root.

use crate::FsError;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Lexical join of `root` + relative path; rejects absolute / prefix / root components.
pub fn lexical_join(root: &Path, relative: &str) -> Result<PathBuf, FsError> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(FsError::Forbidden);
    }
    for c in rel.components() {
        match c {
            Component::Normal(_) | Component::CurDir | Component::ParentDir => {}
            Component::RootDir | Component::Prefix(_) => return Err(FsError::Forbidden),
        }
    }

    let mut out = root.to_path_buf();
    if relative.is_empty() || relative == "." {
        return Ok(out);
    }
    for c in rel.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() || !out.starts_with(root) {
                    return Err(FsError::Forbidden);
                }
            }
            Component::Normal(s) => out.push(s),
            Component::RootDir | Component::Prefix(_) => return Err(FsError::Forbidden),
        }
    }
    if !out.starts_with(root) {
        return Err(FsError::Forbidden);
    }
    Ok(out)
}

/// Resolve `relative` under `root` (canonical). Existing paths are canonicalized;
/// missing paths canonicalize the deepest existing ancestor + remaining components.
pub async fn resolve(root: &Path, relative: &str) -> Result<PathBuf, FsError> {
    let lexical = lexical_join(root, relative)?;

    match tokio::fs::canonicalize(&lexical).await {
        Ok(canon) => {
            if !canon.starts_with(root) {
                return Err(FsError::Forbidden);
            }
            Ok(canon)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            resolve_missing(root, lexical).await
        }
        Err(e) => Err(e.into()),
    }
}

async fn resolve_missing(root: &Path, lexical: PathBuf) -> Result<PathBuf, FsError> {
    let mut cur = lexical.clone();
    let mut suffix: Vec<OsString> = Vec::new();

    loop {
        match tokio::fs::canonicalize(&cur).await {
            Ok(canon) => {
                if !canon.starts_with(root) {
                    return Err(FsError::Forbidden);
                }
                let mut out = canon;
                for part in suffix.iter().rev() {
                    out.push(part);
                }
                if !out.starts_with(root) {
                    return Err(FsError::Forbidden);
                }
                return Ok(out);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let name = cur
                    .file_name()
                    .ok_or(FsError::Forbidden)?
                    .to_os_string();
                if !cur.pop() {
                    return Err(FsError::Forbidden);
                }
                if !cur.starts_with(root) && cur != root {
                    // still walking within lexical root tree
                    if !lexical_under(root, &cur) {
                        return Err(FsError::Forbidden);
                    }
                }
                suffix.push(name);
            }
            Err(e) => return Err(e.into()),
        }
    }
}

fn lexical_under(root: &Path, path: &Path) -> bool {
    path.starts_with(root) || path == root
}

/// Relative display path (forward slashes) from jail root.
pub fn rel_display(root: &Path, absolute: &Path) -> String {
    absolute
        .strip_prefix(root)
        .unwrap_or(absolute)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_absolute() {
        let root = Path::new("/tmp/jail");
        assert!(matches!(
            lexical_join(root, "/etc/passwd"),
            Err(FsError::Forbidden)
        ));
    }

    #[test]
    fn rejects_escape() {
        let root = PathBuf::from("/tmp/jail");
        assert!(matches!(
            lexical_join(&root, "../outside"),
            Err(FsError::Forbidden)
        ));
        assert!(matches!(
            lexical_join(&root, "a/../../outside"),
            Err(FsError::Forbidden)
        ));
    }

    #[test]
    fn allows_nested() {
        let root = PathBuf::from("/tmp/jail");
        let p = lexical_join(&root, "a/b/c.txt").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/jail/a/b/c.txt"));
    }

    #[tokio::test]
    async fn resolve_existing_and_missing() {
        let dir = tempfile::tempdir().unwrap();
        let root = tokio::fs::canonicalize(dir.path()).await.unwrap();
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("notes/a.txt"), b"hi").unwrap();

        let got = resolve(&root, "notes/a.txt").await.unwrap();
        assert!(got.ends_with("notes/a.txt"));

        let missing = resolve(&root, "notes/new.txt").await.unwrap();
        assert!(missing.starts_with(&root));
        assert!(missing.ends_with("notes/new.txt"));

        assert!(matches!(
            resolve(&root, "../etc/passwd").await,
            Err(FsError::Forbidden)
        ));
    }
}
