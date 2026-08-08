use include_dir::{include_dir, Dir};

pub static API_TEMPLATE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/api");
pub static WEB_TEMPLATE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/web");
pub static MIN_TEMPLATE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/minimal");

pub mod codegen;
