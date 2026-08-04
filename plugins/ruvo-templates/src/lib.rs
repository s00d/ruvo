//! Template engines for Ruvo.

use minijinja::{Environment, Value};
use ruvo_core::{Error, Response, Result};
use serde::Serialize;

/// Pluggable template engine.
pub trait TemplateEngine: Send + Sync {
    fn render(&self, name: &str, ctx: Value) -> Result<String>;
}

/// MiniJinja-backed engine.
#[derive(Clone)]
pub struct MiniJinjaEngine {
    env: Environment<'static>,
}

impl MiniJinjaEngine {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    pub fn add_template(&mut self, name: &str, source: &str) -> Result<()> {
        self.env
            .add_template_owned(name.to_string(), source.to_string())
            .map_err(|e| Error::Internal(format!("template: {e}")))?;
        Ok(())
    }

    pub fn render_html<S: Serialize>(&self, name: &str, ctx: S) -> Result<Response> {
        let value = Value::from_serialize(&ctx);
        let html = TemplateEngine::render(self, name, value)?;
        Ok(Response::html(html))
    }
}

impl Default for MiniJinjaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine for MiniJinjaEngine {
    fn render(&self, name: &str, ctx: Value) -> Result<String> {
        let tmpl = self
            .env
            .get_template(name)
            .map_err(|e| Error::Internal(format!("template: {e}")))?;
        tmpl.render(ctx)
            .map_err(|e| Error::Internal(format!("render: {e}")))
    }
}
