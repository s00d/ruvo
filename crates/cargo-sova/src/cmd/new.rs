use clap::{ArgAction, Args};
use include_dir::Dir;
use std::fs;
use std::path::{Path, PathBuf};

use crate::templates;
use crate::util::{io_err, path_err, sanitize_crate_name};

#[derive(Args, Debug)]
pub struct NewArgs {
    pub name: String,
    #[arg(long, action = ArgAction::SetTrue)]
    pub api: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub web: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub minimal: bool,
}

pub fn run(args: NewArgs) -> Result<(), String> {
    let selected = [args.api, args.web, args.minimal]
        .into_iter()
        .filter(|v| *v)
        .count();
    if selected > 1 {
        return Err("use only one of --api/--web/--minimal".into());
    }
    let template = if args.web {
        &templates::WEB_TEMPLATE
    } else if args.minimal {
        &templates::MIN_TEMPLATE
    } else {
        &templates::API_TEMPLATE
    };
    let root = PathBuf::from(&args.name);
    if root.exists() {
        return Err(format!("directory already exists: {}", root.display()));
    }
    let crate_name = sanitize_crate_name(&args.name)?;
    write_dir_template(template, &root, &crate_name)?;
    println!("created {} (package `{crate_name}`)", root.display());
    Ok(())
}

fn write_dir_template(dir: &Dir<'_>, root: &Path, app_name: &str) -> Result<(), String> {
    write_dir_template_rec(dir, dir, root, app_name)
}

fn write_dir_template_rec(
    root_dir: &Dir<'_>,
    dir: &Dir<'_>,
    root: &Path,
    app_name: &str,
) -> Result<(), String> {
    fs::create_dir_all(root).map_err(io_err)?;
    for file in dir.files() {
        let rel = file.path().strip_prefix(root_dir.path()).map_err(path_err)?;
        let out = root.join(rel);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        let src = file.contents_utf8().ok_or_else(|| {
            format!("template file is not valid UTF-8: {}", file.path().display())
        })?;
        let rendered = src.replace("{{name}}", app_name);
        fs::write(out, rendered).map_err(io_err)?;
    }
    for subdir in dir.dirs() {
        write_dir_template_rec(root_dir, subdir, root, app_name)?;
    }
    Ok(())
}
