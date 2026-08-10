//! Send notifications to users / audiences.

use crate::channel::{Channel, Via};
use crate::entity;
use crate::list::NotificationRow;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use sova_core::{Error, EventBus, Request, Result};
use sova_db::{DbError, DbExt, DbHandle};
use std::collections::HashMap;
use std::sync::Arc;

/// Recipient marker on the request (or derived from CurrentUser when `auth` feature).
#[derive(Clone, Copy, Debug)]
pub struct NotificationUser(pub i64);

/// App-state service installed by [`super::Notifications`].
#[derive(Clone)]
pub struct NotificationService {
    pub(crate) channels: Arc<HashMap<String, Channel>>,
}

impl NotificationService {
    pub fn channel(&self, slug: &str) -> Option<&Channel> {
        self.channels.get(slug)
    }

    pub fn ensure_channel(&self, slug: &str) -> Result<&Channel> {
        self.channels
            .get(slug)
            .ok_or_else(|| Error::BadRequest(format!("unknown notification channel `{slug}`")))
    }
}

/// Fluent notification builder.
pub struct Notify {
    recipients: Recipients,
    channel: String,
    event: String,
    title: String,
    body: Option<String>,
    data: Value,
    vias: Vec<Via>,
    /// When true, skip publish-permission checks (trusted server code).
    system: bool,
}

enum Recipients {
    Users(Vec<i64>),
    #[cfg(feature = "auth")]
    Role(String),
    #[cfg(feature = "auth")]
    Permission(String),
}

impl Notify {
    pub fn to(user_id: i64) -> Self {
        Self::to_many([user_id])
    }

    pub fn to_many(ids: impl IntoIterator<Item = i64>) -> Self {
        Self {
            recipients: Recipients::Users(ids.into_iter().collect()),
            channel: "default".into(),
            event: "notification".into(),
            title: String::new(),
            body: None,
            data: Value::Object(Default::default()),
            vias: default_vias(),
            system: true,
        }
    }

    #[cfg(feature = "auth")]
    pub fn to_role(slug: impl Into<String>) -> Self {
        Self {
            recipients: Recipients::Role(slug.into()),
            channel: "default".into(),
            event: "notification".into(),
            title: String::new(),
            body: None,
            data: Value::Object(Default::default()),
            vias: default_vias(),
            system: true,
        }
    }

    #[cfg(feature = "auth")]
    pub fn to_permission(slug: impl Into<String>) -> Self {
        Self {
            recipients: Recipients::Permission(slug.into()),
            channel: "default".into(),
            event: "notification".into(),
            title: String::new(),
            body: None,
            data: Value::Object(Default::default()),
            vias: default_vias(),
            system: true,
        }
    }

    pub fn channel(mut self, slug: impl Into<String>) -> Self {
        self.channel = slug.into();
        self
    }

    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = event.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    pub fn via(mut self, vias: impl IntoIterator<Item = Via>) -> Self {
        self.vias = vias.into_iter().collect();
        self
    }

    /// Mark as system send (default for builders) — skips channel publish ACL.
    pub fn system(mut self, yes: bool) -> Self {
        self.system = yes;
        self
    }

    /// HTTP / untrusted send: enforce channel publish permission when set.
    pub fn as_user(mut self) -> Self {
        self.system = false;
        self
    }

    pub async fn send(self, req: &Request) -> Result<Vec<NotificationRow>> {
        let svc = req
            .try_state::<NotificationService>()
            .ok_or_else(|| Error::Internal("Notifications plugin not installed".into()))?;
        let ch = svc.ensure_channel(&self.channel)?;

        if !self.system {
            if let Some(ref perm) = ch.publish {
                #[cfg(feature = "auth")]
                {
                    use sova_auth::AuthExt;
                    req.require_permission(perm)?;
                }
                #[cfg(not(feature = "auth"))]
                {
                    let _ = perm;
                    return Err(Error::custom(
                        403,
                        "channel publish ACL requires notifications-auth",
                    ));
                }
            }
        }

        let user_ids = resolve_recipients(req.db(), &self.recipients).await?;
        let channel = self.channel.clone();
        let event = self.event.clone();
        let mut out = Vec::with_capacity(user_ids.len());
        for uid in user_ids.iter().copied() {
            match deliver_one(req, uid, &self).await {
                Ok(Some(row)) => out.push(row),
                Ok(None) => {}
                Err(e) => tracing::warn!(error = %e, user_id = uid, "notification deliver failed"),
            }
        }
        if let Some(bus) = req.try_state::<EventBus>() {
            bus.dispatch(crate::NotificationSent {
                channel,
                event,
                recipients: user_ids,
            });
        }
        Ok(out)
    }
}

fn default_vias() -> Vec<Via> {
    #[cfg(feature = "ws")]
    {
        vec![Via::Database, Via::Ws]
    }
    #[cfg(not(feature = "ws"))]
    {
        vec![Via::Database]
    }
}

async fn resolve_recipients(db: &DbHandle, recipients: &Recipients) -> Result<Vec<i64>> {
    match recipients {
        Recipients::Users(ids) => {
            let _ = db;
            Ok(ids.clone())
        }
        #[cfg(feature = "auth")]
        Recipients::Role(slug) => crate::audience::user_ids_with_role(db, slug).await,
        #[cfg(feature = "auth")]
        Recipients::Permission(slug) => crate::audience::user_ids_with_permission(db, slug).await,
    }
}

async fn deliver_one(req: &Request, user_id: i64, n: &Notify) -> Result<Option<NotificationRow>> {
    let mut row = None;
    for via in &n.vias {
        match via {
            Via::Database => {
                row = Some(insert_row(req.db(), user_id, n).await?);
            }
            #[cfg(feature = "ws")]
            Via::Ws => {
                push_ws(req, user_id, row.as_ref(), n).await;
            }
            #[cfg(feature = "mail")]
            Via::Mail => {
                send_mail(req, user_id, n).await;
            }
        }
    }
    Ok(row)
}

async fn insert_row(db: &DbHandle, user_id: i64, n: &Notify) -> Result<NotificationRow> {
    let props = serde_json::to_string(&n.data).unwrap_or_else(|_| "{}".into());
    let model = entity::ActiveModel {
        user_id: Set(user_id),
        channel: Set(n.channel.clone()),
        event: Set(n.event.clone()),
        title: Set(n.title.clone()),
        body: Set(n.body.clone()),
        data: Set(props),
        read_at: Set(None),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    let inserted = model
        .insert(db)
        .await
        .map_err(|e| Error::from(DbError::from(e)))?;
    Ok(NotificationRow::from(inserted))
}

#[cfg(feature = "ws")]
async fn push_ws(req: &Request, user_id: i64, row: Option<&NotificationRow>, n: &Notify) {
    let Some(ws) = req.try_state::<sova_ws::WsShared>() else {
        return;
    };
    let payload = if let Some(r) = row {
        serde_json::to_string(r).unwrap_or_default()
    } else {
        serde_json::json!({
            "user_id": user_id,
            "channel": n.channel,
            "event": n.event,
            "title": n.title,
            "body": n.body,
            "data": n.data,
        })
        .to_string()
    };
    ws.hub
        .broadcast(
            &format!("user:{user_id}"),
            sova_ws::Message::Text(payload.into()),
        )
        .await;
}

#[cfg(feature = "mail")]
async fn send_mail(req: &Request, _user_id: i64, n: &Notify) {
    use sova_mail::{Email, MailClient};
    let Some(client) = req.try_state::<MailClient>() else {
        tracing::warn!("Via::Mail requested but Mail plugin missing");
        return;
    };
    let Some(to) = n.data.get("email").and_then(|v| v.as_str()) else {
        tracing::warn!("Via::Mail skipped: data.email missing");
        return;
    };
    let text = n.body.clone().unwrap_or_else(|| n.title.clone());
    let html = format!("<p>{}</p>", html_escape(&text));
    let _ = client
        .send(Email::new().to(to).subject(&n.title).text(text).html(html))
        .await;
}

#[cfg(feature = "mail")]
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Request helpers.
pub trait NotifyExt {
    fn notifications(&self) -> Option<Arc<NotificationService>>;
}

impl NotifyExt for Request {
    fn notifications(&self) -> Option<Arc<NotificationService>> {
        self.try_state::<NotificationService>()
    }
}
