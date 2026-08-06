use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;

use crate::templates::codegen::{
    append_migration_mod, append_pub_mod, parse_fields, render_crud_dto, render_crud_handlers,
    render_crud_mod, render_crud_routes, render_entity, render_migration, pluralize,
};
use crate::util::{io_err, to_type_name, utc_ymdhms, validate_ident};

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
    Crud { name: String },
}

pub fn run(args: GenerateArgs) -> Result<(), String> {
    match args.kind {
        GenerateKind::Module { name } => generate_module(&name),
        GenerateKind::Plugin { name } => generate_plugin(&name),
        GenerateKind::Model { name, fields } => generate_model(&name, &fields),
        GenerateKind::Crud { name } => generate_crud(&name),
    }
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
    let content = "use ruvo::App;\n\npub fn register(app: &mut App) {\n    // register module routes here\n}\n";
    fs::write(&module_file, content).map_err(io_err)?;
    register_module(name)?;
    println!("generated module `{name}`");
    Ok(())
}

fn generate_plugin(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    let root = PathBuf::from("plugins").join(format!("ruvo-{name}"));
    if root.exists() {
        return Err(format!("plugin already exists: {}", root.display()));
    }
    fs::create_dir_all(root.join("src")).map_err(io_err)?;
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"ruvo-{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nruvo-core = {{ path = \"../../crates/ruvo-core\" }}\n"
        ),
    )
    .map_err(io_err)?;
    fs::write(
        root.join("src/lib.rs"),
        format!(
            "use ruvo_core::{{App, Plugin}};\n\npub struct {};\n\nimpl Plugin for {} {{\n    fn install(self, _app: &mut App) {{}}\n}}\n",
            to_type_name(name),
            to_type_name(name)
        ),
    )
    .map_err(io_err)?;
    println!("generated plugin `ruvo-{name}`");
    Ok(())
}

fn generate_model(name: &str, fields: &str) -> Result<(), String> {
    validate_ident(name)?;
    let specs = parse_fields(fields)?;
    let entity_path = PathBuf::from("src/entities").join(format!("{name}.rs"));
    if entity_path.exists() {
        return Err(format!("entity already exists: {}", entity_path.display()));
    }
    fs::create_dir_all("src/entities").map_err(io_err)?;
    fs::write(&entity_path, render_entity(name, &specs)).map_err(io_err)?;
    append_pub_mod("src/entities/mod.rs", name)?;

    let stamp = utc_ymdhms();
    let mig_mod = format!("m{stamp}_create_{name}");
    let mig_path = PathBuf::from("src/migrations").join(format!("{mig_mod}.rs"));
    fs::create_dir_all("src/migrations").map_err(io_err)?;
    fs::write(&mig_path, render_migration(name, &specs)).map_err(io_err)?;
    append_migration_mod(&mig_mod)?;

    println!("generated model `{name}` + migration `{mig_mod}`");
    Ok(())
}

fn generate_crud(name: &str) -> Result<(), String> {
    validate_ident(name)?;
    let module_file = PathBuf::from("src/modules").join(format!("{name}.rs"));
    let module_dir = PathBuf::from("src/modules").join(name);
    if module_file.exists() || module_dir.exists() {
        return Err(format!("module already exists: {name}"));
    }
    fs::create_dir_all(&module_dir).map_err(io_err)?;

    let plural = pluralize(name);
    fs::write(module_dir.join("mod.rs"), render_crud_mod(name, &plural)).map_err(io_err)?;
    fs::write(module_dir.join("dto.rs"), render_crud_dto()).map_err(io_err)?;
    fs::write(module_dir.join("handlers.rs"), render_crud_handlers(name)).map_err(io_err)?;
    fs::write(module_dir.join("routes.rs"), render_crud_routes(&plural)).map_err(io_err)?;

    register_module(name)?;
    println!("generated crud module `{name}` at /{plural}");
    Ok(())
}

fn register_module(name: &str) -> Result<(), String> {
    let registry = PathBuf::from("src/modules/mod.rs");
    if !registry.exists() {
        if let Some(parent) = registry.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        fs::write(&registry, "use ruvo::App;\n\npub fn register(_app: &mut App) {}\n").map_err(io_err)?;
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
        next.push_str("\nuse ruvo::App;\n\npub fn register(app: &mut App) {\n");
        next.push_str(&format!("{mount_line}\n"));
        next.push_str("}\n");
    }
    fs::write(&registry, next).map_err(io_err)?;
    Ok(())
}
