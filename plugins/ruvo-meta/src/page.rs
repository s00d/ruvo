//! Per-route meta as [`RouteValue`].

use ruvo_core::extend::RouteValue;
use std::borrow::Cow;

/// Partial page meta attached via `.with(MetaPage::…)`.
#[derive(Debug, Clone, Default)]
pub struct MetaPage {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub noindex: bool,
    pub canonical_path: Option<String>,
    pub moved_to: Option<String>,
    pub og_type: Option<String>,
    /// Skip automatic HTML head injection for this route.
    pub manual: bool,
}

impl MetaPage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    pub fn image(mut self, i: impl Into<String>) -> Self {
        self.image = Some(i.into());
        self
    }

    pub fn noindex(mut self) -> Self {
        self.noindex = true;
        self
    }

    pub fn canonical_path(mut self, p: impl Into<String>) -> Self {
        self.canonical_path = Some(p.into());
        self
    }

    pub fn moved_to(mut self, p: impl Into<String>) -> Self {
        self.moved_to = Some(p.into());
        self
    }

    pub fn og_type(mut self, t: impl Into<String>) -> Self {
        self.og_type = Some(t.into());
        self
    }

    pub fn manual(mut self) -> Self {
        self.manual = true;
        self
    }
}

impl RouteValue for MetaPage {
    fn label(&self) -> Cow<'static, str> {
        if self.noindex {
            Cow::Borrowed("MetaPage(noindex)")
        } else if let Some(ref t) = self.title {
            Cow::Owned(format!("MetaPage({t})"))
        } else {
            Cow::Borrowed("MetaPage")
        }
    }
}
