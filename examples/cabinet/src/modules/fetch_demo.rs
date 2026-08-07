use ruvo::vld;
use ruvo::{
    doc_schema, CsrfExt, CurrentUser, HttpExt, Meta, RenderExt, Request, Response, Result, Router,
    ValidExt, ValidateRouteExt,
};
use serde_json::json;

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct FetchForm {
        pub url: String => vld::string().min(8).max(500),
        pub csrf: String => vld::string().min(8),
    }
}

doc_schema!(FetchForm);

pub fn mount(r: &mut Router) {
    r.get("/fetch", form_get).with(Meta::noindex());
    r.post("/fetch", form_post).validate_form::<FetchForm>();
}

async fn form_get(req: Request) -> Result<Response> {
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();
    let csrf = req.csrf_token();
    Ok(req.render(
        "cabinet/fetch.html",
        json!({
            "user": { "name": user.name },
            "csrf": csrf,
            "result": null,
        }),
    )?)
}

async fn form_post(req: Request) -> Result<Response> {
    let form = req.valid::<FetchForm>().clone();
    let user = req.get::<CurrentUser>().expect("CurrentUser").clone();

    let result = match req.http().get(&form.url).send().await {
        Ok(res) => {
            let status = res.status_u16();
            let body = res.text().unwrap_or_else(|_| String::from("(unreadable body)"));
            let preview: String = body.chars().take(400).collect();
            json!({ "ok": true, "status": status, "preview": preview })
        }
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    };

    let csrf = req.csrf_token();
    Ok(req.render(
        "cabinet/fetch.html",
        json!({
            "user": { "name": user.name },
            "csrf": csrf,
            "result": result,
            "url": form.url,
        }),
    )?)
}
