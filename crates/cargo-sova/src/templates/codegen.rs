use std::fs;
use std::path::PathBuf;

use crate::util::{io_err, validate_ident};

#[derive(Clone, Debug)]
pub struct FieldSpec {
    pub name: String,
    pub ty: FieldType,
    pub nullable: bool,
    pub unique: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum FieldType {
    String,
    Text,
    Int,
    BigInt,
    Bool,
    Uuid,
    Float,
    DateTime,
}

pub fn parse_fields(raw: &str) -> Result<Vec<FieldSpec>, String> {
    let mut out = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let pieces: Vec<&str> = part.split(':').map(str::trim).collect();
        if pieces.len() < 2 {
            return Err(format!(
                "bad field `{part}` — expected name:type[:unique]"
            ));
        }
        let name = pieces[0].to_string();
        validate_ident(&name)?;
        let mut type_token = pieces[1].to_string();
        let mut nullable = false;
        if type_token.ends_with('?') {
            nullable = true;
            type_token.pop();
        }
        let mut unique = false;
        for extra in &pieces[2..] {
            match *extra {
                "unique" => unique = true,
                "?" => nullable = true,
                other => {
                    return Err(format!("unknown field modifier `{other}` on `{name}`"));
                }
            }
        }
        let ty = parse_field_type(&type_token)?;
        out.push(FieldSpec {
            name,
            ty,
            nullable,
            unique,
        });
    }
    if out.is_empty() {
        return Err("--fields must list at least one column".into());
    }
    Ok(out)
}

fn parse_field_type(raw: &str) -> Result<FieldType, String> {
    match raw {
        "string" => Ok(FieldType::String),
        "text" => Ok(FieldType::Text),
        "int" => Ok(FieldType::Int),
        "bigint" => Ok(FieldType::BigInt),
        "bool" => Ok(FieldType::Bool),
        "uuid" => Ok(FieldType::Uuid),
        "float" => Ok(FieldType::Float),
        "datetime" => Ok(FieldType::DateTime),
        other => Err(format!(
            "unknown type `{other}` (string|text|int|bigint|bool|uuid|float|datetime)"
        )),
    }
}

fn rust_type(spec: &FieldSpec) -> String {
    let base = match spec.ty {
        FieldType::String | FieldType::Text => "String",
        FieldType::Int => "i32",
        FieldType::BigInt => "i64",
        FieldType::Bool => "bool",
        FieldType::Uuid => "Uuid",
        FieldType::Float => "f64",
        FieldType::DateTime => "DateTimeWithTimeZone",
    };
    if spec.nullable {
        format!("Option<{base}>")
    } else {
        base.to_string()
    }
}

fn migration_col_helper(spec: &FieldSpec) -> String {
    let base = match spec.ty {
        FieldType::String => "string",
        FieldType::Text => "text",
        FieldType::Int => "integer",
        FieldType::BigInt => "big_integer",
        FieldType::Bool => "boolean",
        FieldType::Uuid => "uuid",
        FieldType::Float => "double",
        FieldType::DateTime => "timestamp_with_time_zone",
    };
    let helper = if spec.unique {
        format!("{base}_uniq")
    } else if spec.nullable {
        format!("{base}_null")
    } else {
        base.to_string()
    };
    format!("{helper}(\"{}\")", spec.name)
}

pub fn render_entity(name: &str, fields: &[FieldSpec]) -> String {
    let mut out = String::new();
    out.push_str("use sea_orm::entity::prelude::*;\n");
    out.push_str("use serde::{Deserialize, Serialize};\n\n");
    out.push_str("#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]\n");
    out.push_str(&format!("#[sea_orm(table_name = \"{name}\")]\n"));
    out.push_str("pub struct Model {\n");
    out.push_str("    #[sea_orm(primary_key)]\n");
    out.push_str("    pub id: i32,\n");
    for f in fields {
        if f.unique {
            out.push_str("    #[sea_orm(unique)]\n");
        }
        out.push_str(&format!("    pub {}: {},\n", f.name, rust_type(f)));
    }
    out.push_str("}\n\n");
    out.push_str("#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]\n");
    out.push_str("pub enum Relation {}\n\n");
    out.push_str("impl ActiveModelBehavior for ActiveModel {}\n");
    out
}

pub fn render_migration(name: &str, fields: &[FieldSpec]) -> String {
    let mut out = String::new();
    out.push_str("use sea_orm_migration::{prelude::*, schema::*};\n\n");
    out.push_str("#[derive(DeriveMigrationName)]\n");
    out.push_str("pub struct Migration;\n\n");
    out.push_str("#[async_trait::async_trait]\n");
    out.push_str("impl MigrationTrait for Migration {\n");
    out.push_str("    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {\n");
    out.push_str("        manager\n");
    out.push_str("            .create_table(\n");
    out.push_str("                Table::create()\n");
    out.push_str(&format!("                    .table(\"{name}\")\n"));
    out.push_str("                    .if_not_exists()\n");
    out.push_str("                    .col(pk_auto(\"id\"))\n");
    for f in fields {
        out.push_str(&format!("                    .col({})\n", migration_col_helper(f)));
    }
    out.push_str("                    .to_owned(),\n");
    out.push_str("            )\n");
    out.push_str("            .await\n");
    out.push_str("    }\n\n");
    out.push_str("    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {\n");
    out.push_str("        manager\n");
    out.push_str(&format!(
        "            .drop_table(Table::drop().table(\"{name}\").to_owned())\n"
    ));
    out.push_str("            .await\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

pub fn render_blank_migration() -> String {
    r#"use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager;
        Ok(())
    }
}
"#
    .to_string()
}

pub fn render_seed(snake: &str) -> String {
    let _ = snake;
    r#"use sova::Error;
use std::sync::Arc;

pub async fn run(state: Arc<sova::extend::StateMap>) -> Result<(), Error> {
    let _pool = state
        .get::<sova::DbPool>()
        .ok_or_else(|| Error::Internal("DbPool missing".into()))?;
    // seed rows here
    Ok(())
}
"#
    .to_string()
}

/// Rewrite `src/seeds/mod.rs` with `pub mod` + composed `run`.
pub fn ensure_seeds_registry(snake: &str) -> Result<(), String> {
    let path = PathBuf::from("src/seeds/mod.rs");
    let mut mods = Vec::new();
    if path.exists() {
        let existing = fs::read_to_string(&path).map_err(io_err)?;
        for line in existing.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("pub mod ") {
                if let Some(name) = rest.strip_suffix(';') {
                    let name = name.trim();
                    if !name.is_empty() && !mods.iter().any(|m: &String| m == name) {
                        mods.push(name.to_string());
                    }
                }
            }
        }
    }
    if mods.iter().any(|m| m == snake) {
        return Err(format!("seed `{snake}` already registered in {}", path.display()));
    }
    mods.push(snake.to_string());

    let mut out = String::new();
    for m in &mods {
        out.push_str(&format!("pub mod {m};\n"));
    }
    out.push_str("\nuse sova::Error;\nuse std::sync::Arc;\n\n");
    out.push_str("/// Wire with `Db::from_env().seed(crate::seeds::run)`.\n");
    out.push_str("pub async fn run(state: Arc<sova::extend::StateMap>) -> Result<(), Error> {\n");
    for m in &mods {
        out.push_str(&format!("    {m}::run(state.clone()).await?;\n"));
    }
    out.push_str("    Ok(())\n}\n");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    fs::write(&path, out).map_err(io_err)?;
    Ok(())
}

pub fn render_crud_mod(name: &str, plural: &str) -> String {
    format!(
        "mod dto;\nmod handlers;\nmod routes;\n\nuse sova::App;\n\npub fn register(app: &mut App) {{\n    app.mount(\"/{plural}\", routes::routes());\n}}\n\n// entity: crate::entities::{name}\n"
    )
}

fn dto_fields(fields: Option<&[FieldSpec]>) -> Vec<FieldSpec> {
    match fields {
        Some(f) if !f.is_empty() => f.to_vec(),
        _ => vec![FieldSpec {
            name: "name".into(),
            ty: FieldType::String,
            nullable: false,
            unique: false,
        }],
    }
}

fn vld_expr(spec: &FieldSpec) -> String {
    let base = match spec.ty {
        FieldType::String | FieldType::Uuid => "vld::string().min(1)".to_string(),
        FieldType::Text => "vld::string()".to_string(),
        FieldType::Int | FieldType::BigInt => "vld::number().int()".to_string(),
        FieldType::Float => "vld::number()".to_string(),
        FieldType::Bool => "vld::boolean()".to_string(),
        FieldType::DateTime => "vld::string().min(1)".to_string(),
    };
    if spec.nullable {
        format!("{base}.optional()")
    } else {
        base
    }
}

fn dto_rust_type(spec: &FieldSpec) -> String {
    match spec.ty {
        FieldType::String | FieldType::Text | FieldType::Uuid | FieldType::DateTime => {
            if spec.nullable {
                "Option<String>".into()
            } else {
                "String".into()
            }
        }
        FieldType::Int | FieldType::BigInt => {
            if spec.nullable {
                "Option<i64>".into()
            } else {
                "i64".into()
            }
        }
        FieldType::Float => {
            if spec.nullable {
                "Option<f64>".into()
            } else {
                "f64".into()
            }
        }
        FieldType::Bool => {
            if spec.nullable {
                "Option<bool>".into()
            } else {
                "bool".into()
            }
        }
    }
}

fn set_expr(spec: &FieldSpec, from: &str) -> String {
    match spec.ty {
        FieldType::Int => {
            if spec.nullable {
                format!("Set({from}.{}.map(|v| v as i32))", spec.name)
            } else {
                format!("Set({from}.{} as i32)", spec.name)
            }
        }
        FieldType::Uuid | FieldType::DateTime => {
            if spec.nullable {
                format!(
                    "Set(match {from}.{} {{ Some(s) => Some(s.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?), None => None }})",
                    spec.name, spec.name
                )
            } else {
                format!(
                    "Set({from}.{}.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?)",
                    spec.name, spec.name
                )
            }
        }
        _ => format!("Set({from}.{})", spec.name),
    }
}

pub fn render_crud_dto(fields: Option<&[FieldSpec]>) -> String {
    let fields = dto_fields(fields);
    let mut out = String::new();
    out.push_str("use sova::doc_schema;\n\n");
    out.push_str("vld::schema! {\n");
    out.push_str("    #[derive(Debug, Clone, serde::Serialize)]\n");
    out.push_str("    pub struct Create {\n");
    for f in &fields {
        out.push_str(&format!(
            "        pub {}: {} => {},\n",
            f.name,
            dto_rust_type(f),
            vld_expr(f)
        ));
    }
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("vld::schema! {\n");
    out.push_str("    #[derive(Debug, Clone, serde::Serialize)]\n");
    out.push_str("    pub struct Update {\n");
    for f in &fields {
        out.push_str(&format!(
            "        pub {}: {} => {},\n",
            f.name,
            dto_rust_type(f),
            vld_expr(f)
        ));
    }
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("vld::schema! {\n");
    out.push_str("    #[derive(Debug, Clone)]\n");
    out.push_str("    pub struct IdParams {\n");
    out.push_str("        pub id: String => vld::string().min(1),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("doc_schema!(Create, Update, IdParams);\n");
    out
}

pub fn render_crud_handlers(name: &str, fields: Option<&[FieldSpec]>) -> String {
    let fields = dto_fields(fields);
    let mut out = String::new();
    out.push_str("use super::dto::{Create, IdParams, Update};\n");
    out.push_str(&format!("use crate::entities::{name} as entity;\n"));
    out.push_str("use sova::{\n");
    out.push_str(
        "    ActiveModelTrait, DbError, DbExt, Error, Json, Request, Response, Result, Set, ValidationExt,\n",
    );
    out.push_str("};\n");
    out.push_str("use sea_orm::EntityTrait;\n\n");

    out.push_str("pub async fn list(req: Request) -> Result<Json<Vec<entity::Model>>> {\n");
    out.push_str("    let rows = entity::Entity::find()\n");
    out.push_str("        .all(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?;\n");
    out.push_str("    Ok(Json(rows))\n");
    out.push_str("}\n\n");

    out.push_str("pub async fn create(mut req: Request) -> Result<(u16, Json<entity::Model>)> {\n");
    out.push_str("    let body: Create = req.validate().await?;\n");
    out.push_str("    let row = entity::ActiveModel {\n");
    for f in &fields {
        out.push_str(&format!("        {}: {},\n", f.name, set_expr(f, "body")));
    }
    out.push_str("        ..Default::default()\n");
    out.push_str("    }\n");
    out.push_str("    .insert(req.db())\n");
    out.push_str("    .await\n");
    out.push_str("    .map_err(DbError::from)?;\n");
    out.push_str("    Ok((201, Json(row)))\n");
    out.push_str("}\n\n");

    out.push_str("pub async fn show(req: Request) -> Result<Json<entity::Model>> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str("    Ok(Json(row))\n");
    out.push_str("}\n\n");

    out.push_str("pub async fn update(mut req: Request) -> Result<Json<entity::Model>> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let body: Update = req.validate().await?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str("    let mut am: entity::ActiveModel = row.into();\n");
    for f in &fields {
        out.push_str(&format!("    am.{} = {};\n", f.name, set_expr(f, "body")));
    }
    out.push_str("    Ok(Json(am.update(req.db()).await.map_err(DbError::from)?))\n");
    out.push_str("}\n\n");

    out.push_str("pub async fn destroy(req: Request) -> Result<Response> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str("    let am: entity::ActiveModel = row.into();\n");
    out.push_str("    am.delete(req.db()).await.map_err(DbError::from)?;\n");
    out.push_str("    Ok(Response::empty().status(204))\n");
    out.push_str("}\n");
    out
}

pub fn render_crud_routes(_plural: &str) -> String {
    [
        "use super::dto::{Create, IdParams, Update};\n",
        "use super::handlers;\n",
        "use sova::{Doc, DocVldExt, OpenApiDocExt, Router};\n\n",
        "pub fn routes() -> Router {\n",
        "    let mut r = Router::new();\n",
        "    r.get(\"/\", handlers::list).doc(Doc::new());\n",
        "    r.post(\"/\", handlers::create)\n",
        "        .doc(Doc::new().body::<Create>().created_schema(serde_json::json!({ \"type\": \"object\" })));\n",
        "    r.get(\"/:id\", handlers::show)\n",
        "        .doc(Doc::new().params::<IdParams>());\n",
        "    r.put(\"/:id\", handlers::update)\n",
        "        .doc(Doc::new().params::<IdParams>().body::<Update>());\n",
        "    r.delete(\"/:id\", handlers::destroy)\n",
        "        .doc(Doc::new().params::<IdParams>());\n",
        "    r\n",
        "}\n",
    ]
    .concat()
}

pub fn pluralize(name: &str) -> String {
    if name.ends_with('s') {
        name.to_string()
    } else {
        format!("{name}s")
    }
}

/// `Welcome` → type `WelcomeMail`, snake `welcome`.
pub fn mailer_names(raw: &str) -> (String, String) {
    let snake = crate::util::to_snake_case(raw);
    let ty_base = crate::util::to_type_name(&snake);
    let ty = if ty_base.ends_with("Mail") {
        ty_base
    } else {
        format!("{ty_base}Mail")
    };
    (ty, snake)
}

pub fn render_mailer(ty: &str, snake: &str) -> String {
    let title = ty.trim_end_matches("Mail");
    format!(
        r#"use sova::{{Content, Envelope, Mailable}};
use serde_json::json;

pub struct {ty} {{
    pub name: String,
}}

impl Mailable for {ty} {{
    fn envelope(&self) -> Envelope {{
        Envelope::new("{title}")
    }}

    fn content(&self) -> Content {{
        Content::view(
            "mail/{snake}.html",
            json!({{ "name": self.name }}),
        )
    }}
}}
"#
    )
}

pub fn render_mail_view(title: &str, _snake: &str) -> String {
    format!(
        r#"{{% extends "mail/layout.html" %}}
{{% block title %}}{title}{{% endblock %}}
{{% block content %}}
<p>Hello {{{{ name }}}}!</p>
{{% endblock %}}
"#
    )
}

pub fn render_mail_layout() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>{% block title %}Sova{% endblock %}</title>
</head>
<body style="font-family: system-ui, sans-serif; line-height: 1.5; color: #111;">
  {% block content %}{% endblock %}
</body>
</html>
"#
    .to_string()
}

pub fn ensure_mail_layout() -> Result<(), String> {
    let path = PathBuf::from("views/mail/layout.html");
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    fs::write(&path, render_mail_layout()).map_err(io_err)?;
    Ok(())
}

pub fn render_job(snake: &str) -> String {
    format!(
        r#"use sova::Job;

pub fn job() -> Job {{
    Job::new("{snake}", |task| async move {{
        let _payload = task.payload;
        Ok(())
    }})
}}
"#
    )
}

/// Rewrite `src/jobs/mod.rs` with `pub mod` lines + `install(tasks)` chain.
pub fn ensure_jobs_registry(snake: &str) -> Result<(), String> {
    let path = PathBuf::from("src/jobs/mod.rs");
    let mut mods = Vec::new();
    if path.exists() {
        let existing = fs::read_to_string(&path).map_err(io_err)?;
        for line in existing.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("pub mod ") {
                if let Some(name) = rest.strip_suffix(';') {
                    let name = name.trim();
                    if !name.is_empty() && !mods.iter().any(|m: &String| m == name) {
                        mods.push(name.to_string());
                    }
                }
            }
        }
    }
    if mods.iter().any(|m| m == snake) {
        return Err(format!("job `{snake}` already registered in {}", path.display()));
    }
    mods.push(snake.to_string());

    let mut out = String::new();
    for m in &mods {
        out.push_str(&format!("pub mod {m};\n"));
    }
    out.push_str("\nuse sova::Tasks;\n\n");
    out.push_str("pub fn install(tasks: Tasks) -> Tasks {\n");
    out.push_str("    tasks");
    for m in &mods {
        out.push_str(&format!("\n        .job({m}::job())"));
    }
    out.push_str("\n}\n");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    fs::write(&path, out).map_err(io_err)?;
    Ok(())
}

pub fn render_web_mod(name: &str, plural: &str) -> String {
    format!(
        "mod dto;\nmod handlers;\nmod routes;\n\nuse sova::App;\n\npub fn register(app: &mut App) {{\n    app.mount(\"/{plural}\", routes::routes());\n}}\n\n// entity: crate::entities::{name}\n"
    )
}

pub fn render_web_dto(fields: Option<&[FieldSpec]>) -> String {
    let fields = dto_fields(fields);
    let mut out = String::new();
    out.push_str("use sova::doc_schema;\n\n");
    out.push_str("vld::schema! {\n");
    out.push_str("    #[derive(Debug, Clone)]\n");
    out.push_str("    pub struct Form {\n");
    for f in &fields {
        // HTML forms submit strings.
        let (ty, vld) = if f.nullable {
            ("Option<String>", "vld::string().optional()")
        } else if matches!(f.ty, FieldType::Text | FieldType::Bool) {
            ("String", "vld::string()")
        } else {
            ("String", "vld::string().min(1)")
        };
        out.push_str(&format!("        pub {}: {} => {},\n", f.name, ty, vld));
    }
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("vld::schema! {\n");
    out.push_str("    #[derive(Debug, Clone)]\n");
    out.push_str("    pub struct IdParams {\n");
    out.push_str("        pub id: String => vld::string().min(1),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("doc_schema!(Form, IdParams);\n");
    out
}

fn web_set_from_form(spec: &FieldSpec) -> String {
    match spec.ty {
        FieldType::String | FieldType::Text => format!("Set(form.{}.clone())", spec.name),
        FieldType::Int => {
            if spec.nullable {
                format!(
                    "Set(form.{}.as_ref().and_then(|s| s.parse::<i32>().ok()))",
                    spec.name
                )
            } else {
                format!(
                    "Set(form.{}.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?)",
                    spec.name, spec.name
                )
            }
        }
        FieldType::BigInt => {
            if spec.nullable {
                format!(
                    "Set(form.{}.as_ref().and_then(|s| s.parse::<i64>().ok()))",
                    spec.name
                )
            } else {
                format!(
                    "Set(form.{}.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?)",
                    spec.name, spec.name
                )
            }
        }
        FieldType::Float => {
            if spec.nullable {
                format!(
                    "Set(form.{}.as_ref().and_then(|s| s.parse::<f64>().ok()))",
                    spec.name
                )
            } else {
                format!(
                    "Set(form.{}.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?)",
                    spec.name, spec.name
                )
            }
        }
        FieldType::Bool => {
            if spec.nullable {
                format!(
                    "Set(form.{}.as_ref().map(|s| matches!(s.as_str(), \"1\" | \"true\" | \"on\" | \"yes\")))",
                    spec.name
                )
            } else {
                format!(
                    "Set(matches!(form.{}.as_str(), \"1\" | \"true\" | \"on\" | \"yes\"))",
                    spec.name
                )
            }
        }
        FieldType::Uuid | FieldType::DateTime => {
            if spec.nullable {
                format!(
                    "Set(match &form.{} {{ Some(s) => Some(s.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?), None => None }})",
                    spec.name, spec.name
                )
            } else {
                format!(
                    "Set(form.{}.parse().map_err(|_| Error::BadRequest(\"invalid {}\".into()))?)",
                    spec.name, spec.name
                )
            }
        }
    }
}

pub fn render_web_handlers(name: &str, plural: &str, fields: Option<&[FieldSpec]>) -> String {
    let fields = dto_fields(fields);
    let mut out = String::new();
    out.push_str("use super::dto::{Form, IdParams};\n");
    out.push_str(&format!("use crate::entities::{name} as entity;\n"));
    out.push_str("use sova::{\n");
    out.push_str("    ActiveModelTrait, DbError, DbExt, Error, Redirect, RenderExt, Request, Response, Result,\n");
    out.push_str("    Set, ValidationExt,\n");
    out.push_str("};\n");
    out.push_str("use sea_orm::EntityTrait;\n");
    out.push_str("use serde_json::json;\n\n");

    out.push_str("pub async fn index(req: Request) -> Result<Response> {\n");
    out.push_str("    let rows = entity::Entity::find()\n");
    out.push_str("        .all(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?;\n");
    out.push_str(&format!(
        "    Ok(req.render(\"{plural}/index.html\", json!({{ \"items\": rows }}))?)\n"
    ));
    out.push_str("}\n\n");

    out.push_str("pub async fn show(req: Request) -> Result<Response> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str(&format!(
        "    Ok(req.render(\"{plural}/show.html\", json!({{ \"item\": row }}))?)\n"
    ));
    out.push_str("}\n\n");

    out.push_str("pub async fn new_form(req: Request) -> Result<Response> {\n");
    out.push_str(&format!(
        "    Ok(req.render(\"{plural}/form.html\", json!({{ \"item\": serde_json::Value::Null, \"action\": \"/{plural}\" }}))?)\n"
    ));
    out.push_str("}\n\n");

    out.push_str("pub async fn create(mut req: Request) -> Result<Response> {\n");
    out.push_str("    let form: Form = req.validate().await?;\n");
    out.push_str("    let row = entity::ActiveModel {\n");
    for f in &fields {
        out.push_str(&format!("        {}: {},\n", f.name, web_set_from_form(f)));
    }
    out.push_str("        ..Default::default()\n");
    out.push_str("    }\n");
    out.push_str("    .insert(req.db())\n");
    out.push_str("    .await\n");
    out.push_str("    .map_err(DbError::from)?;\n");
    out.push_str(&format!(
        "    Ok(Redirect::see_other(format!(\"/{plural}/{{}}\", row.id)).into_response())\n"
    ));
    out.push_str("}\n\n");

    out.push_str("pub async fn edit_form(req: Request) -> Result<Response> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str(&format!(
        "    Ok(req.render(\"{plural}/form.html\", json!({{ \"item\": row, \"action\": format!(\"/{plural}/{{}}\", id) }}))?)\n"
    ));
    out.push_str("}\n\n");

    out.push_str("pub async fn update(mut req: Request) -> Result<Response> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let form: Form = req.validate().await?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str("    let mut am: entity::ActiveModel = row.into();\n");
    for f in &fields {
        out.push_str(&format!("    am.{} = {};\n", f.name, web_set_from_form(f)));
    }
    out.push_str("    let row = am.update(req.db()).await.map_err(DbError::from)?;\n");
    out.push_str(&format!(
        "    Ok(Redirect::see_other(format!(\"/{plural}/{{}}\", row.id)).into_response())\n"
    ));
    out.push_str("}\n\n");

    out.push_str("pub async fn destroy(req: Request) -> Result<Response> {\n");
    out.push_str("    let params: IdParams = req.validate_params()?;\n");
    out.push_str("    let id: i32 = params\n");
    out.push_str("        .id\n");
    out.push_str("        .parse()\n");
    out.push_str("        .map_err(|_| Error::BadRequest(\"invalid id\".into()))?;\n");
    out.push_str("    let row = entity::Entity::find_by_id(id)\n");
    out.push_str("        .one(req.db())\n");
    out.push_str("        .await\n");
    out.push_str("        .map_err(DbError::from)?\n");
    out.push_str("        .ok_or(Error::NotFound)?;\n");
    out.push_str("    let am: entity::ActiveModel = row.into();\n");
    out.push_str("    am.delete(req.db()).await.map_err(DbError::from)?;\n");
    out.push_str(&format!(
        "    Ok(Redirect::see_other(\"/{plural}\").into_response())\n"
    ));
    out.push_str("}\n\n");
    out.push_str("use sova::IntoResponse;\n");
    out
}

pub fn render_web_routes(_plural: &str) -> String {
    [
        "use super::dto::{Form, IdParams};\n",
        "use super::handlers;\n",
        "use sova::{Doc, DocVldExt, OpenApiDocExt, Router};\n\n",
        "pub fn routes() -> Router {\n",
        "    let mut r = Router::new();\n",
        "    r.get(\"/\", handlers::index).doc(Doc::new());\n",
        "    r.get(\"/new\", handlers::new_form).doc(Doc::new());\n",
        "    r.post(\"/\", handlers::create)\n",
        "        .doc(Doc::new().body::<Form>());\n",
        "    r.get(\"/:id\", handlers::show)\n",
        "        .doc(Doc::new().params::<IdParams>());\n",
        "    r.get(\"/:id/edit\", handlers::edit_form)\n",
        "        .doc(Doc::new().params::<IdParams>());\n",
        "    r.post(\"/:id\", handlers::update)\n",
        "        .doc(Doc::new().params::<IdParams>().body::<Form>());\n",
        "    r.post(\"/:id/delete\", handlers::destroy)\n",
        "        .doc(Doc::new().params::<IdParams>());\n",
        "    r\n",
        "}\n",
    ]
    .concat()
}

pub fn render_layout_stub() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>{% block title %}Sova{% endblock %}</title>
</head>
<body>
  {% block content %}{% endblock %}
</body>
</html>
"#
    .to_string()
}

pub fn ensure_root_layout() -> Result<(), String> {
    let path = PathBuf::from("views/layout.html");
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    fs::write(&path, render_layout_stub()).map_err(io_err)?;
    Ok(())
}

pub fn render_web_index_view(plural: &str, title: &str) -> String {
    format!(
        r#"{{% extends "layout.html" %}}
{{% block title %}}{title}{{% endblock %}}
{{% block content %}}
<h1>{title}</h1>
<p><a href="/{plural}/new">New</a></p>
<ul>
{{% for item in items %}}
  <li><a href="/{plural}/{{{{ item.id }}}}">#{{{{ item.id }}}}</a></li>
{{% endfor %}}
</ul>
{{% endblock %}}
"#
    )
}

pub fn render_web_show_view(plural: &str, title: &str) -> String {
    format!(
        r#"{{% extends "layout.html" %}}
{{% block title %}}{title}{{% endblock %}}
{{% block content %}}
<h1>{title} #{{{{ item.id }}}}</h1>
<p><a href="/{plural}">Back</a> · <a href="/{plural}/{{{{ item.id }}}}/edit">Edit</a></p>
<form method="post" action="/{plural}/{{{{ item.id }}}}/delete">
  <button type="submit">Delete</button>
</form>
{{% endblock %}}
"#
    )
}

pub fn render_web_form_view(plural: &str, title: &str, fields: Option<&[FieldSpec]>) -> String {
    let fields = dto_fields(fields);
    let mut inputs = String::new();
    for f in &fields {
        inputs.push_str(&format!(
            r#"  <p>
    <label>{name}<br>
      <input name="{name}" value="{{{{ item.{name} if item else '' }}}}">
    </label>
  </p>
"#,
            name = f.name
        ));
    }
    format!(
        r#"{{% extends "layout.html" %}}
{{% block title %}}{title}{{% endblock %}}
{{% block content %}}
<h1>{title}</h1>
<form method="post" action="{{{{ action }}}}">
{inputs}  <button type="submit">Save</button>
</form>
<p><a href="/{plural}">Cancel</a></p>
{{% endblock %}}
"#
    )
}

pub fn render_resource_test(name: &str, plural: &str, api: bool) -> String {
    let kind = if api { "api" } else { "resource" };
    let path = if api {
        format!("/{plural}")
    } else {
        format!("/{plural}")
    };
    format!(
        r#"//! Smoke scaffold for `{name}` {kind}.
//!
//! Wire your App factory (Db / Templates / modules::register), then un-ignore:
//!
//! ```ignore
//! #[tokio::test]
//! #[ignore = "requires app factory + migrations"]
//! async fn {name}_{kind}_smoke() {{
//!     let app = /* build App */;
//!     let c = sova::TestClient::tracked(app).unwrap();
//!     let res = c.get("{path}").await;
//!     res.assert_status(200);
//! }}
//! ```
"#
    )
}

pub fn append_pub_mod(path: &str, name: &str) -> Result<(), String> {
    let path = PathBuf::from(path);
    let line = format!("pub mod {name};\n");
    if path.exists() {
        let existing = fs::read_to_string(&path).map_err(io_err)?;
        if existing.contains(&format!("pub mod {name};")) {
            return Err(format!("`{name}` already in {}", path.display()));
        }
        let mut next = existing;
        if !next.ends_with('\n') && !next.is_empty() {
            next.push('\n');
        }
        next.push_str(&line);
        fs::write(&path, next).map_err(io_err)?;
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        fs::write(&path, line).map_err(io_err)?;
    }
    Ok(())
}

pub fn append_migration_mod(mig_mod: &str) -> Result<(), String> {
    let path = PathBuf::from("src/migrations/mod.rs");
    let mod_line = format!("mod {mig_mod};");
    let box_line = format!("            Box::new({mig_mod}::Migration),");

    if !path.exists() {
        let content = format!(
            "pub use sea_orm_migration::prelude::*;\n\n{mod_line}\n\npub struct Migrator;\n\n#[async_trait::async_trait]\nimpl MigratorTrait for Migrator {{\n    fn migrations() -> Vec<Box<dyn MigrationTrait>> {{\n        vec![\n{box_line}\n        ]\n    }}\n}}\n"
        );
        fs::write(&path, content).map_err(io_err)?;
        return Ok(());
    }

    let existing = fs::read_to_string(&path).map_err(io_err)?;
    if existing.contains(&mod_line) {
        return Err(format!("migration `{mig_mod}` already registered"));
    }

    let mut next = existing;
    if let Some(idx) = next.find("pub use sea_orm_migration::prelude::*;") {
        let insert_at = idx + "pub use sea_orm_migration::prelude::*;\n".len();
        next.insert_str(insert_at, &format!("\n{mod_line}\n"));
    } else {
        next.insert_str(0, &format!("{mod_line}\n"));
    }

    if let Some(idx) = next.find("vec![") {
        let insert_at = idx + "vec![".len();
        next.insert_str(insert_at, &format!("\n{box_line}"));
    } else {
        next.push_str(&format!(
            "\npub use sea_orm_migration::prelude::*;\n\npub struct Migrator;\n\n#[async_trait::async_trait]\nimpl MigratorTrait for Migrator {{\n    fn migrations() -> Vec<Box<dyn MigrationTrait>> {{\n        vec![\n{box_line}\n        ]\n    }}\n}}\n"
        ));
    }

    fs::write(&path, next).map_err(io_err)?;
    Ok(())
}
