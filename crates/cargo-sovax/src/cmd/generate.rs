use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;

use crate::manifest::{ensure_mail_stack, ensure_resource_stack, ensure_tasks_stack};
use crate::templates::codegen::{
    append_migration_mod, append_pub_mod, ensure_jobs_registry, ensure_mail_layout,
    ensure_root_layout, ensure_seeds_registry, mailer_names, parse_fields, pluralize,
    render_blank_migration, render_crud_dto, render_crud_handlers, render_crud_mod,
    render_crud_routes, render_entity, render_job, render_mail_view, render_mailer,
    render_migration, render_resource_test, render_seed, render_web_dto, render_web_form_view,
    render_web_handlers, render_web_index_view, render_web_mod, render_web_routes,
    render_web_show_view, FieldSpec,
};
use crate::util::{io_err, to_snake_case, to_type_name, utc_ymdhms, validate_ident};

#[derive(Args, Debug)]
pub struct GenerateArgs {
    #[command(subcommand)]
    pub kind: GenerateKind,
}

#[derive(Subcommand, Debug)]
pub enum GenerateKind {
    Module { name: String },
    Plugin { name: String },
    Model {
        name: String,
        #[arg(long)]
        fields: String,
    },
    /// JSON REST module (alias of `resource --api`).
    Crud { name: String },
    Mailer { name: String },
    Job { name: String },
    /// Alias of [`GenerateKind::Job`].
    Worker { name: String },
    Resource {
        name: String,
        #[arg(long)]
        fields: Option<String>,
        /// JSON REST + smoke test instead of HTML views.
        #[arg(long)]
        api: bool,
    },
    /// Standalone SeaORM migration (`--fields` → create table; else empty up/down).
    Migration {
        name: String,
        #[arg(long)]
        fields: Option<String>,
    },
    /// Seed function under `src/seeds/` (compose via `seeds::run`).
    Seed { name: String },
}

pub fn run(args: GenerateArgs) -> Result<(), String> {
    refuse_workspace_root()?;
    match args.kind {
        GenerateKind::Module { name } => generate_module(&name),
        GenerateKind::Plugin { name } => generate_plugin(&name),
        GenerateKind::Model { name, fields } => generate_model(&name, &fields),
        GenerateKind::Crud { name } => generate_resource(&name, None, true),
        GenerateKind::Mailer { name } => generate_mailer(&name),
        GenerateKind::Job { name } | GenerateKind::Worker { name } => generate_job(&name),
        GenerateKind::Resource { name, fields, api } => {
            generate_resource(&name, fields.as_deref(), api)
        }
        GenerateKind::Migration { name, fields } => {
            generate_migration(&name, fields.as_deref())
        }
        GenerateKind::Seed { name } => generate_seed(&name),
    }
}

fn refuse_workspace_root() -> Result<(), String> {
    let Ok(raw) = fs::read_to_string("Cargo.toml") else {
        return Ok(());
    };
    if raw.contains("[workspace]") && !raw.contains("[package]") {
        return Err(
            "refusing to generate in a Cargo workspace root — cd into an app package first".into(),
        );
    }
    Ok(())
}

fn generate_module(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    let module_file = PathBuf::from("src/modules").join(format!("{name}.rs"));
    let module_dir = PathBuf::from("src/modules").join(name);
    if module_file.exists() || module_dir.exists() {
        return Err(format!("module already exists: {name}"));
    }
    if let Some(parent) = module_file.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let content = "use sova::App;\n\npub fn register(app: &mut App) {\n    // register module routes here\n}\n";
    fs::write(&module_file, content).map_err(io_err)?;
    register_module(name)?;
    println!("generated module `{name}`");
    Ok(())
}

fn generate_plugin(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    let root = PathBuf::from("plugins").join(format!("sova-{name}"));
    if root.exists() {
        return Err(format!("plugin already exists: {}", root.display()));
    }
    fs::create_dir_all(root.join("src")).map_err(io_err)?;

    let ty = to_type_name(name);
    let display = ty
        .chars()
        .flat_map(|c| {
            if c.is_uppercase() {
                vec![' ', c]
            } else {
                vec![c]
            }
        })
        .collect::<String>()
        .trim()
        .to_string();

    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "sova-{name}"
version = "0.1.0"
edition = "2021"
description = "{display} plugin for Sova"
license = "MIT"
repository = "https://github.com/s00d/sova"
homepage = "https://s00d.github.io/sova/"
keywords = ["sova", "plugin", "{name}"]

[dependencies]
sova-core = {{ version = "0.1.0", path = "../../crates/sova-core" }}
"#
        ),
    )
    .map_err(io_err)?;

    fs::write(
        root.join("src/lib.rs"),
        format!(
            r#"//! {display} — Sova plugin.
//!
//! ```ignore
//! use sova_{name}::{ty};
//!
//! app.install({ty}::new());
//! ```

use sova_core::extend::with_leaked;
use sova_core::{{App, Plugin, PluginMeta, Request, Response}};

/// {display} plugin.
///
/// Demo install: adds an `x-sova-{name}` response header and stores config in app state.
pub struct {ty} {{
    /// Value written to the `x-sova-{name}` header.
    header_value: String,
}}

impl {ty} {{
    pub fn new() -> Self {{
        Self {{
            header_value: "1".into(),
        }}
    }}

    pub fn header_value(mut self, value: impl Into<String>) -> Self {{
        self.header_value = value.into();
        self
    }}
}}

impl Default for {ty} {{
    fn default() -> Self {{
        Self::new()
    }}
}}

impl Plugin for {ty} {{
    fn id(&self) -> &'static str {{
        "{name}"
    }}

    // fn requires(&self) -> &'static [&'static str] {{
    //     &["cookies"] // install those plugins first
    // }}

    fn meta(&self) -> PluginMeta {{
        PluginMeta::new("{display}")
            .description("Example plugin generated by cargo sovax generate plugin")
            .version(env!("CARGO_PKG_VERSION"))
            // .sdk(sova_core::PLUGIN_SDK_VERSION) // default; pin only if you need an older SDK
    }}

    fn install(self, app: &mut App) {{
        app.state({ty}Config {{
            header_value: self.header_value.clone(),
        }});

        app.use_middleware(with_leaked(self, |plugin, req: Request, next| async move {{
            let mut res: Response = next(req).await;
            res = res.header("x-sova-{name}", &plugin.header_value);
            res
        }}));

        // Routes (optional):
        // app.get("/{name}/ping", || async {{ Response::text("pong") }});
    }}
}}

/// Shared config inserted by [`{ty}::install`].
#[derive(Clone, Debug)]
pub struct {ty}Config {{
    pub header_value: String,
}}
"#
        ),
    )
    .map_err(io_err)?;

    fs::write(
        root.join("README.md"),
        format!(
            r#"# sova-{name}

{display} plugin for [Sova](../../README.md).

## Usage

```rust
use sova_{name}::{ty};

let mut app = sova_core::App::new();
app.install({ty}::new());
```

Declares Plugin SDK via `PluginMeta` (see `sova_core::PLUGIN_SDK_VERSION`).
Workspace members `plugins/*` pick this crate up automatically.
"#
        ),
    )
    .map_err(io_err)?;

    println!("generated plugin `sova-{name}`");
    println!("  path: {}", root.display());
    println!("  use:  app.install(sova_{name}::{ty}::new());");
    Ok(())
}

fn write_model(name: &str, specs: &[FieldSpec]) -> Result<(), String> {
    let entity_path = PathBuf::from("src/entities").join(format!("{name}.rs"));
    if entity_path.exists() {
        return Err(format!("entity already exists: {}", entity_path.display()));
    }
    fs::create_dir_all("src/entities").map_err(io_err)?;
    fs::write(&entity_path, render_entity(name, specs)).map_err(io_err)?;
    append_pub_mod("src/entities/mod.rs", name)?;

    let stamp = utc_ymdhms();
    let mig_mod = format!("m{stamp}_create_{name}");
    let mig_path = PathBuf::from("src/migrations").join(format!("{mig_mod}.rs"));
    fs::create_dir_all("src/migrations").map_err(io_err)?;
    fs::write(&mig_path, render_migration(&mig_mod, name, specs)).map_err(io_err)?;
    append_migration_mod(&mig_mod)?;

    println!("generated model `{name}` + migration `{mig_mod}`");
    Ok(())
}

fn generate_model(name: &str, fields: &str) -> Result<(), String> {
    validate_ident(name)?;
    ensure_resource_stack(false)?;
    let specs = parse_fields(fields)?;
    write_model(name, &specs)
}

fn generate_mailer(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    ensure_mail_stack()?;
    let (ty, snake) = mailer_names(name);
    let file = PathBuf::from("src/mailers").join(format!("{snake}.rs"));
    if file.exists() {
        return Err(format!("mailer already exists: {}", file.display()));
    }
    fs::create_dir_all("src/mailers").map_err(io_err)?;
    fs::write(&file, render_mailer(&ty, &snake)).map_err(io_err)?;
    append_pub_mod("src/mailers/mod.rs", &snake)?;

    ensure_mail_layout()?;
    let view = PathBuf::from("views/mail").join(format!("{snake}.html"));
    if !view.exists() {
        fs::write(&view, render_mail_view(ty.trim_end_matches("Mail"), &snake)).map_err(io_err)?;
    }

    println!("generated mailer `{ty}`");
    println!("  src/mailers/{snake}.rs");
    println!("  views/mail/{snake}.html");
    println!("  use:  req.mail().to(user).send_mail({ty} {{ name: \"…\".into() }}).await?;");
    Ok(())
}

fn generate_job(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    ensure_tasks_stack()?;
    let snake = to_snake_case(name);
    validate_ident(&snake)?;
    let file = PathBuf::from("src/jobs").join(format!("{snake}.rs"));
    if file.exists() {
        return Err(format!("job already exists: {}", file.display()));
    }
    fs::create_dir_all("src/jobs").map_err(io_err)?;
    fs::write(&file, render_job(&snake)).map_err(io_err)?;
    ensure_jobs_registry(&snake)?;

    println!("generated job `{snake}`");
    println!("  src/jobs/{snake}.rs");
    println!("  use:  let tasks = crate::jobs::install(Tasks::new(/* store */));");
    Ok(())
}

fn generate_migration(name: &str, fields: Option<&str>) -> Result<(), String> {
    ensure_resource_stack(false)?;
    let snake = to_snake_case(name);
    validate_ident(&snake)?;
    let stamp = utc_ymdhms();
    let mig_mod = format!("m{stamp}_{snake}");
    let mig_path = PathBuf::from("src/migrations").join(format!("{mig_mod}.rs"));
    if mig_path.exists() {
        return Err(format!("migration already exists: {}", mig_path.display()));
    }
    fs::create_dir_all("src/migrations").map_err(io_err)?;

    let body = if let Some(raw) = fields {
        let specs = parse_fields(raw)?;
        let table = snake
            .strip_prefix("create_")
            .unwrap_or(snake.as_str());
        validate_ident(table)?;
        render_migration(&mig_mod, table, &specs)
    } else {
        render_blank_migration(&mig_mod)
    };
    fs::write(&mig_path, body).map_err(io_err)?;
    append_migration_mod(&mig_mod)?;

    println!("generated migration `{mig_mod}`");
    if fields.is_some() {
        let table = snake.strip_prefix("create_").unwrap_or(snake.as_str());
        println!("  create table `{table}`");
    }
    Ok(())
}

fn generate_seed(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    ensure_resource_stack(false)?;
    let snake = to_snake_case(name);
    validate_ident(&snake)?;
    let file = PathBuf::from("src/seeds").join(format!("{snake}.rs"));
    if file.exists() {
        return Err(format!("seed already exists: {}", file.display()));
    }
    fs::create_dir_all("src/seeds").map_err(io_err)?;
    fs::write(&file, render_seed(&snake)).map_err(io_err)?;
    ensure_seeds_registry(&snake)?;

    println!("generated seed `{snake}`");
    println!("  src/seeds/{snake}.rs");
    println!("  use:  Db::from_env().seed(crate::seeds::run)");
    Ok(())
}

fn generate_resource(name: &str, fields: Option<&str>, api: bool) -> Result<(), String> {
    validate_ident(name)?;
    ensure_resource_stack(api)?;
    let specs = match fields {
        Some(raw) => Some(parse_fields(raw)?),
        None => None,
    };
    if let Some(ref specs) = specs {
        write_model(name, specs)?;
    }

    let module_file = PathBuf::from("src/modules").join(format!("{name}.rs"));
    let module_dir = PathBuf::from("src/modules").join(name);
    if module_file.exists() || module_dir.exists() {
        return Err(format!("module already exists: {name}"));
    }
    fs::create_dir_all(&module_dir).map_err(io_err)?;

    let plural = pluralize(name);
    let field_slice = specs.as_deref();

    if api {
        fs::write(module_dir.join("mod.rs"), render_crud_mod(name, &plural)).map_err(io_err)?;
        fs::write(module_dir.join("dto.rs"), render_crud_dto(field_slice)).map_err(io_err)?;
        fs::write(
            module_dir.join("handlers.rs"),
            render_crud_handlers(name, field_slice),
        )
        .map_err(io_err)?;
        fs::write(module_dir.join("routes.rs"), render_crud_routes(&plural)).map_err(io_err)?;

        fs::create_dir_all("tests").map_err(io_err)?;
        let test_path = PathBuf::from("tests").join(format!("{name}_api.rs"));
        if !test_path.exists() {
            fs::write(&test_path, render_resource_test(name, &plural, true)).map_err(io_err)?;
        }

        register_module(name)?;
        println!("generated api resource `{name}` at /{plural}");
        println!("  tests/{name}_api.rs");
        println!("  hint: DATABASE_URL=sqlite:./app.db?mode=rwc cargo run");
    } else {
        let title = to_type_name(name);
        fs::write(module_dir.join("mod.rs"), render_web_mod(name, &plural)).map_err(io_err)?;
        fs::write(module_dir.join("dto.rs"), render_web_dto(field_slice)).map_err(io_err)?;
        fs::write(
            module_dir.join("handlers.rs"),
            render_web_handlers(name, &plural, field_slice),
        )
        .map_err(io_err)?;
        fs::write(module_dir.join("routes.rs"), render_web_routes(&plural)).map_err(io_err)?;

        ensure_root_layout()?;
        let views = PathBuf::from("views").join(&plural);
        fs::create_dir_all(&views).map_err(io_err)?;
        fs::write(
            views.join("index.html"),
            render_web_index_view(&plural, &title),
        )
        .map_err(io_err)?;
        fs::write(
            views.join("show.html"),
            render_web_show_view(&plural, &title),
        )
        .map_err(io_err)?;
        fs::write(
            views.join("form.html"),
            render_web_form_view(&plural, &title, field_slice),
        )
        .map_err(io_err)?;

        fs::create_dir_all("tests").map_err(io_err)?;
        let test_path = PathBuf::from("tests").join(format!("{name}_resource.rs"));
        if !test_path.exists() {
            fs::write(&test_path, render_resource_test(name, &plural, false)).map_err(io_err)?;
        }

        register_module(name)?;
        println!("generated web resource `{name}` at /{plural}");
        println!("  views/{plural}/{{index,show,form}}.html");
        println!("  tests/{name}_resource.rs");
        println!("  hint: DATABASE_URL=sqlite:./app.db?mode=rwc cargo run");
    }
    Ok(())
}

fn register_module(name: &str) -> Result<(), String> {
    let registry = PathBuf::from("src/modules/mod.rs");
    if !registry.exists() {
        if let Some(parent) = registry.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        fs::write(&registry, "use sova::App;\n\npub fn register(_app: &mut App) {}\n").map_err(io_err)?;
    }
    let existing = fs::read_to_string(&registry).map_err(io_err)?;
    let mod_line = format!("pub mod {name};");
    let mount_line = format!("    {name}::register(app);");
    if existing.contains(&mod_line) {
        return Err(format!(
            "module `{name}` is already registered in {}",
            registry.display()
        ));
    }
    let mut next = String::new();
    next.push_str(&format!("{mod_line}\n"));
    next.push_str(&existing);
    if let Some(idx) = next.find("pub fn register(app: &mut App) {") {
        let insert_at = idx + "pub fn register(app: &mut App) {\n".len();
        next.insert_str(insert_at, &format!("{mount_line}\n"));
    } else if let Some(idx) = next.find("pub fn register(_app: &mut App) {") {
        next.replace_range(
            idx..idx + "pub fn register(_app: &mut App) {}".len(),
            &format!("pub fn register(app: &mut App) {{\n{mount_line}\n}}"),
        );
    } else {
        next.push_str("\nuse sova::App;\n\npub fn register(app: &mut App) {\n");
        next.push_str(&format!("{mount_line}\n"));
        next.push_str("}\n");
    }
    fs::write(&registry, next).map_err(io_err)?;
    Ok(())
}
