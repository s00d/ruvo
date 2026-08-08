//! WebSocket inbox push (feature `ws`).

use ruvo_core::extend::MwEntry;
use ruvo_core::{App, Error, Request, Response, Router};
use ruvo_ws::{upgrade_ws, Message, WsShared};

use crate::notify::NotificationUser;

pub fn install_ws(app: &mut App, path: &str, guard: Option<MwEntry>) {
    let mut r = Router::new();
    if let Some(g) = guard {
        r.use_middleware(g);
    }
    r.get("/", |req: Request| async move {
        let uid = user_id(&req)?;
        if req.try_state::<WsShared>().is_none() {
            return Err(Error::Internal("Ws plugin not installed".into()));
        }
        match upgrade_ws(req, move |mut session| async move {
            let room = format!("user:{uid}");
            let _join = session.join(&room);
            let _ = session
                .send(Message::Text(
                    serde_json::json!({ "ok": true, "room": room })
                        .to_string()
                        .into(),
                ))
                .await;
            while let Some(Ok(_)) = session.recv().await {}
        })
        .await
        {
            Ok(res) | Err(res) => Ok(res),
        }
    });
    app.mount(path, r);
}

fn user_id(req: &Request) -> Result<i64, Error> {
    if let Some(u) = req.get::<NotificationUser>() {
        return Ok(u.0);
    }
    #[cfg(feature = "auth")]
    {
        use ruvo_auth::AuthExt;
        return Ok(req.require_current_user()?.id);
    }
    #[cfg(not(feature = "auth"))]
    {
        let _ = req;
        Err(Error::Unauthorized)
    }
}

#[allow(dead_code)]
fn _response_type_check() -> Response {
    Response::text("")
}
