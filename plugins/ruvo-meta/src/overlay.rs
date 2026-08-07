//! Per-request meta overlay (`Arc<Mutex<…>>` so middleware sees handler writes).

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
struct MetaOverlayData {
    title: Option<String>,
    description: Option<String>,
    image: Option<String>,
    noindex: Option<bool>,
    canonical_path: Option<String>,
    published: Option<DateTime<Utc>>,
    og_type: Option<String>,
    jsonld: Vec<Value>,
    manual: bool,
}

/// Shared per-request overlay (cheap to clone into middleware after `next`).
#[derive(Debug, Clone, Default)]
pub struct MetaOverlay {
    inner: Arc<Mutex<MetaOverlayData>>,
}

impl MetaOverlay {
    pub fn title(&self, t: impl Into<String>) -> &Self {
        self.inner.lock().unwrap().title = Some(t.into());
        self
    }

    pub fn description(&self, d: impl Into<String>) -> &Self {
        self.inner.lock().unwrap().description = Some(d.into());
        self
    }

    pub fn image(&self, i: impl Into<String>) -> &Self {
        self.inner.lock().unwrap().image = Some(i.into());
        self
    }

    pub fn noindex(&self) -> &Self {
        self.inner.lock().unwrap().noindex = Some(true);
        self
    }

    pub fn canonical_path(&self, p: impl Into<String>) -> &Self {
        self.inner.lock().unwrap().canonical_path = Some(p.into());
        self
    }

    pub fn published(&self, t: DateTime<Utc>) -> &Self {
        self.inner.lock().unwrap().published = Some(t);
        self
    }

    pub fn published_opt(&self, t: Option<DateTime<Utc>>) -> &Self {
        if let Some(t) = t {
            self.published(t);
        }
        self
    }

    pub fn og_type(&self, t: impl Into<String>) -> &Self {
        self.inner.lock().unwrap().og_type = Some(t.into());
        self
    }

    pub fn jsonld(&self, block: Value) -> &Self {
        self.inner.lock().unwrap().jsonld.push(block);
        self
    }

    pub fn manual(&self) -> &Self {
        self.inner.lock().unwrap().manual = true;
        self
    }

    pub(crate) fn snapshot(&self) -> OverlaySnapshot {
        let g = self.inner.lock().unwrap();
        OverlaySnapshot {
            title: g.title.clone(),
            description: g.description.clone(),
            image: g.image.clone(),
            noindex: g.noindex,
            canonical_path: g.canonical_path.clone(),
            published: g.published,
            og_type: g.og_type.clone(),
            jsonld: g.jsonld.clone(),
            manual: g.manual,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OverlaySnapshot {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub noindex: Option<bool>,
    pub canonical_path: Option<String>,
    pub published: Option<DateTime<Utc>>,
    pub og_type: Option<String>,
    pub jsonld: Vec<Value>,
    pub manual: bool,
}
