//! Credential extractors (Bearer, header, query, cookie, custom).

use sova_core::Request;

/// Where a credential was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Authorization,
    Header,
    Query,
    Cookie,
    Custom,
}

/// Extracted credential string.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub scheme: Option<String>,
    pub value: String,
    pub source: Source,
}

impl Credentials {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }
}

type ExtractFn = Arc<dyn Fn(&Request) -> Option<Credentials> + Send + Sync>;

use std::sync::Arc;

/// Ordered chain of extractors — first match wins.
#[derive(Clone)]
pub struct Extract {
    parts: Vec<ExtractFn>,
}

impl Extract {
    pub fn empty() -> Self {
        Self { parts: Vec::new() }
    }

    pub fn bearer() -> Self {
        Self::empty().or_fn(|req| {
            let auth = req.header("authorization")?;
            let token = auth
                .strip_prefix("Bearer ")
                .or_else(|| auth.strip_prefix("bearer "))?
                .trim();
            if token.is_empty() {
                return None;
            }
            Some(Credentials {
                scheme: Some("Bearer".into()),
                value: token.to_string(),
                source: Source::Authorization,
            })
        })
    }

    pub fn header(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::empty().or_fn(move |req| {
            let value = req.header(&name)?.trim();
            if value.is_empty() {
                return None;
            }
            Some(Credentials {
                scheme: None,
                value: value.to_string(),
                source: Source::Header,
            })
        })
    }

    pub fn query(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::empty().or_fn(move |req| {
            let value = req.query.get(&name)?.trim();
            if value.is_empty() {
                return None;
            }
            Some(Credentials {
                scheme: None,
                value: value.to_string(),
                source: Source::Query,
            })
        })
    }

    pub fn cookie(name: impl Into<String>) -> Self {
        let name = name.into();
        Self::empty().or_fn(move |req| {
            let raw = req.header("cookie")?;
            for part in raw.split(';') {
                let part = part.trim();
                if let Some((k, v)) = part.split_once('=') {
                    if k.trim() == name {
                        let value = v.trim();
                        if value.is_empty() {
                            return None;
                        }
                        return Some(Credentials {
                            scheme: None,
                            value: value.to_string(),
                            source: Source::Cookie,
                        });
                    }
                }
            }
            None
        })
    }

    /// Custom extractor.
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(&Request) -> Option<Credentials> + Send + Sync + 'static,
    {
        Self::empty().or_fn(f)
    }

    pub fn or(self, other: Extract) -> Self {
        let mut parts = self.parts;
        parts.extend(other.parts);
        Self { parts }
    }

    fn or_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> Option<Credentials> + Send + Sync + 'static,
    {
        self.parts.push(Arc::new(f));
        self
    }

    pub fn run(&self, req: &Request) -> Option<Credentials> {
        for part in &self.parts {
            if let Some(c) = part(req) {
                return Some(c);
            }
        }
        None
    }
}

impl Default for Extract {
    fn default() -> Self {
        Self::bearer()
    }
}
