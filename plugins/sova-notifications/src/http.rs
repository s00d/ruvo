//! HTTP inbox + broadcast routes.

use crate::channel::Channel;
use crate::list::{
    list_notifications, mark_all_read, mark_read, unread_count, NotificationFilter, NotificationRow,
};
use crate::notify::{NotificationService, NotificationUser, Notify};
use serde::Deserialize;
use serde_json::json;
use sova_core::{Error, Json, Request, Response, Result, Router};
use sova_db::DbExt;

pub fn mount_routes(r: &mut Router) {
    r.get("/", list_handler);
    r.get("/unread-count", unread_handler);
    r.post("/:id/read", mark_one_handler);
    r.post("/read-all", mark_all_handler);
    r.post("/broadcast", broadcast_handler);
}

fn current_user_id(req: &Request) -> Result<i64> {
    if let Some(u) = req.get::<NotificationUser>() {
        return Ok(u.0);
    }
    #[cfg(feature = "auth")]
    {
        use sova_auth::AuthExt;
        return Ok(req.require_current_user()?.id);
    }
    #[cfg(not(feature = "auth"))]
    {
        Err(Error::Unauthorized)
    }
}

fn can_subscribe(req: &Request, ch: &Channel) -> Result<()> {
    let Some(ref perm) = ch.subscribe else {
        return Ok(());
    };
    #[cfg(feature = "auth")]
    {
        use sova_auth::AuthExt;
        req.require_permission(perm)?;
        Ok(())
    }
    #[cfg(not(feature = "auth"))]
    {
        let _ = (req, perm);
        Err(Error::custom(
            403,
            "channel subscribe ACL requires notifications-auth",
        ))
    }
}

async fn list_handler(req: Request) -> Result<Json<Vec<NotificationRow>>> {
    let uid = current_user_id(&req)?;
    let channel = req.query("channel").map(str::to_string);
    if let Some(ref slug) = channel {
        if let Some(svc) = req.try_state::<NotificationService>() {
            if let Some(ch) = svc.channel(slug) {
                can_subscribe(&req, ch)?;
            }
        }
    }
    let unread_only = req
        .query("unread")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let limit = req
        .query("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let rows = list_notifications(
        req.db(),
        uid,
        NotificationFilter {
            channel,
            unread_only,
            limit,
        },
    )
    .await?;
    Ok(Json(rows))
}

async fn unread_handler(req: Request) -> Result<Json<serde_json::Value>> {
    let uid = current_user_id(&req)?;
    let channel = req.query("channel");
    let count = unread_count(req.db(), uid, channel).await?;
    Ok(Json(json!({ "count": count })))
}

async fn mark_one_handler(req: Request) -> Result<Json<serde_json::Value>> {
    let uid = current_user_id(&req)?;
    let id: i64 = req
        .param("id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::BadRequest("id required".into()))?;
    let ok = mark_read(req.db(), uid, id).await?;
    if !ok {
        return Err(Error::NotFound);
    }
    Ok(Json(json!({ "ok": true })))
}

async fn mark_all_handler(req: Request) -> Result<Json<serde_json::Value>> {
    let uid = current_user_id(&req)?;
    let n = mark_all_read(req.db(), uid, req.query("channel")).await?;
    Ok(Json(json!({ "ok": true, "updated": n })))
}

#[derive(Debug, Deserialize)]
struct BroadcastBody {
    channel: String,
    event: String,
    title: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    data: Option<serde_json::Value>,
    audience: AudienceBody,
}

#[derive(Debug, Default, Deserialize)]
struct AudienceBody {
    #[serde(default)]
    users: Option<Vec<i64>>,
    #[cfg(feature = "auth")]
    #[serde(default)]
    role: Option<String>,
    #[cfg(feature = "auth")]
    #[serde(default)]
    permission: Option<String>,
}

async fn broadcast_handler(mut req: Request) -> Result<Response> {
    let body: BroadcastBody = req.json().await?;
    let mut n = if let Some(ids) = body.audience.users {
        Notify::to_many(ids)
    } else {
        #[cfg(feature = "auth")]
        {
            if let Some(role) = body.audience.role {
                Notify::to_role(role)
            } else if let Some(perm) = body.audience.permission {
                Notify::to_permission(perm)
            } else {
                return Err(Error::BadRequest(
                    "audience.users | audience.role | audience.permission required".into(),
                ));
            }
        }
        #[cfg(not(feature = "auth"))]
        {
            return Err(Error::BadRequest("audience.users required".into()));
        }
    };
    n = n
        .channel(body.channel)
        .event(body.event)
        .title(body.title)
        .as_user();
    if let Some(b) = body.body {
        n = n.body(b);
    }
    if let Some(d) = body.data {
        n = n.data(d);
    }
    let rows = n.send(&req).await?;
    Ok(Json(json!({ "ok": true, "sent": rows.len(), "notifications": rows })).into_response())
}

use sova_core::IntoResponse;
