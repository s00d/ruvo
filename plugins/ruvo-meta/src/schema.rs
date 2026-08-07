//! Typed JSON-LD schema.org helpers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};

/// Convert a typed schema to JSON-LD [`Value`].
pub trait ToJsonLd {
    fn json_ld(&self) -> Value;
}

fn wrap<T: Serialize>(ty: &str, value: &T) -> Value {
    let mut v = serde_json::to_value(value).unwrap_or(json!({}));
    if let Value::Object(ref mut map) = v {
        map.insert("@type".into(), Value::String(ty.into()));
        map.insert(
            "@context".into(),
            Value::String("https://schema.org".into()),
        );
    }
    v
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Article {
    pub headline: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_published: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ToJsonLd for Article {
    fn json_ld(&self) -> Value {
        wrap("Article", self)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Product {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
}

impl ToJsonLd for Product {
    fn json_ld(&self) -> Value {
        wrap("Product", self)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Offer {
    pub price: String,
    pub price_currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl ToJsonLd for Offer {
    fn json_ld(&self) -> Value {
        wrap("Offer", self)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ListItem {
    pub position: u32,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BreadcrumbList {
    pub item_list_element: Vec<ListItem>,
}

impl BreadcrumbList {
    pub fn from_pairs(crumbs: &[(&str, &str)]) -> Self {
        Self {
            item_list_element: crumbs
                .iter()
                .enumerate()
                .map(|(i, (name, url))| ListItem {
                    position: (i + 1) as u32,
                    name: (*name).to_string(),
                    item: Some((*url).to_string()),
                })
                .collect(),
        }
    }
}

impl ToJsonLd for BreadcrumbList {
    fn json_ld(&self) -> Value {
        wrap("BreadcrumbList", self)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Organization {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

impl ToJsonLd for Organization {
    fn json_ld(&self) -> Value {
        wrap("Organization", self)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct WebSite {
    pub name: String,
    pub url: String,
}

impl ToJsonLd for WebSite {
    fn json_ld(&self) -> Value {
        wrap("WebSite", self)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FAQPage {
    pub main_entity: Vec<FaqEntry>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FaqEntry {
    pub name: String,
    pub accepted_answer: FaqAnswer,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FaqAnswer {
    pub text: String,
}

impl ToJsonLd for FAQPage {
    fn json_ld(&self) -> Value {
        let entities: Vec<Value> = self
            .main_entity
            .iter()
            .map(|e| {
                json!({
                    "@type": "Question",
                    "name": e.name,
                    "acceptedAnswer": {
                        "@type": "Answer",
                        "text": e.accepted_answer.text,
                    }
                })
            })
            .collect();
        json!({
            "@context": "https://schema.org",
            "@type": "FAQPage",
            "mainEntity": entities,
        })
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LocalBusiness {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telephone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl ToJsonLd for LocalBusiness {
    fn json_ld(&self) -> Value {
        wrap("LocalBusiness", self)
    }
}
