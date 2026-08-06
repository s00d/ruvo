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

pub fn render_crud_mod(name: &str, plural: &str) -> String {
    format!(
        "mod dto;\nmod handlers;\nmod routes;\n\nuse ruvo::App;\n\npub fn register(app: &mut App) {{\n    app.mount(\"/{plural}\", routes::routes());\n}}\n\n// entity: crate::entities::{name}\n"
    )
}

pub fn render_crud_dto() -> String {
    [
        "use ruvo::doc_schema;\n\n",
        "vld::schema! {\n",
        "    #[derive(Debug, Clone, serde::Serialize)]\n",
        "    pub struct Create {\n",
        "        pub name: String => vld::string().min(1),\n",
        "    }\n",
        "}\n\n",
        "vld::schema! {\n",
        "    #[derive(Debug, Clone, serde::Serialize)]\n",
        "    pub struct Update {\n",
        "        pub name: String => vld::string().min(1),\n",
        "    }\n",
        "}\n\n",
        "vld::schema! {\n",
        "    #[derive(Debug, Clone)]\n",
        "    pub struct IdParams {\n",
        "        pub id: String => vld::string().min(1),\n",
        "    }\n",
        "}\n\n",
        "doc_schema!(Create, Update, IdParams);\n",
    ]
    .concat()
}

pub fn render_crud_handlers(name: &str) -> String {
    let mut out = String::new();
    out.push_str("use super::dto::{Create, IdParams, Update};\n");
    out.push_str(&format!("use crate::entities::{name} as entity;\n"));
    out.push_str("use ruvo::{\n");
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
    out.push_str("        name: Set(body.name),\n");
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
    out.push_str("    am.name = Set(body.name);\n");
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
        "use ruvo::{Doc, DocVldExt, OpenApiDocExt, Router};\n\n",
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
