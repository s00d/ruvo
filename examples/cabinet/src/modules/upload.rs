use ruvo::{
    set_avatar, CsrfExt, CurrentUser, DbExt, Redirect, Request, Response, Result, Router,
    ValidateHook,
};
use std::path::PathBuf;

pub fn mount(r: &mut Router) {
    r.post("/avatar", upload_avatar).with(ValidateHook::wrap(|req| {
        Box::pin(async move { Ok(req) })
    }));
}

async fn upload_avatar(mut req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();
    let (csrf_tok, avatar) = {
        let data = req.input().await?;
        let csrf_tok = data.get("csrf").map(str::to_owned);
        let avatar = data
            .file("avatar")
            .cloned()
            .ok_or_else(|| ruvo::Error::BadRequest("avatar required".into()))?;
        (csrf_tok, avatar)
    };

    req.verify_csrf(csrf_tok.as_deref())?;

    if avatar.data.is_empty() {
        return Err(ruvo::Error::BadRequest("empty file".into()).into());
    }
    if avatar.data.len() > 2 * 1024 * 1024 {
        return Err(ruvo::Error::BadRequest("avatar too large (max 2MB)".into()).into());
    }

    let ext = avatar
        .filename
        .as_deref()
        .and_then(|n| PathBuf::from(n).extension().map(|e| e.to_string_lossy().into_owned()))
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp"))
        .unwrap_or_else(|| "bin".into());
    let name = format!("u{}.{}", user.id, ext);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("public")
        .join("uploads");
    avatar.save_in(&root, &name).await?;

    let path = format!("/assets/uploads/{name}");
    set_avatar(req.db(), user.id, Some(path)).await?;
    Ok(Redirect::see_other("/cabinet/profile").into_response())
}

use ruvo::IntoResponse;
