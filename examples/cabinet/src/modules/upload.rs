use ruvo::{
    set_avatar, CsrfExt, CurrentUser, DbExt, Redirect, Request, Response, Result, Router,
    StorageExt, UploadRules, ValidateHook,
};

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

    avatar.validate(
        &UploadRules::new()
            .max_bytes(2 * 1024 * 1024)
            .extensions(["png", "jpg", "jpeg", "gif", "webp"]),
    )?;

    let ext = avatar.extension().unwrap_or_else(|| "bin".into());
    let key = format!("avatars/u{}.{}", user.id, ext);
    let stored = req.storage().store_as(&avatar, &key).await?;
    let path = stored
        .url
        .unwrap_or_else(|| format!("/assets/uploads/{}", stored.key));

    set_avatar(req.db(), user.id, Some(path)).await?;
    Ok(Redirect::see_other("/cabinet/profile").into_response())
}

use ruvo::IntoResponse;
