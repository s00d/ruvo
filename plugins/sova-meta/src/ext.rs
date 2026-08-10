//! `req.meta()` extension.

use crate::overlay::MetaOverlay;
use crate::schema::ToJsonLd;
use crate::{resolve_meta, ResolvedMeta};
use serde_json::Value;
use sova_core::Request;

pub trait MetaExt {
    fn meta(&mut self) -> &MetaOverlay;
    fn resolved_meta(&self) -> ResolvedMeta;
}

impl MetaExt for Request {
    fn meta(&mut self) -> &MetaOverlay {
        if self.get::<MetaOverlay>().is_none() {
            self.set(MetaOverlay::default());
        }
        self.get::<MetaOverlay>().expect("MetaOverlay just set")
    }

    fn resolved_meta(&self) -> ResolvedMeta {
        resolve_meta(self)
    }
}

impl MetaOverlay {
    pub fn jsonld_schema<T: ToJsonLd>(&self, schema: &T) -> &Self {
        self.jsonld(schema.json_ld())
    }

    pub fn jsonld_raw(&self, value: Value) -> &Self {
        self.jsonld(value)
    }
}
