//! CRUD with vld validation + OpenAPI docs at `/docs`.
//!
//! ```bash
//! cargo run -p api_validated
//! ```

use sova::vld;
use sova::{
    doc_schema, App, Cell, Doc, DocVldExt, IntoResponse, Json, OpenApi, OpenApiDocExt, Request,
    Response, Result, ValidationError, ValidationExt,
};
use serde_json::json;

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct CreateUser {
        pub name: String => vld::string().min(2).max(50),
        pub email: String => vld::string().email(),
    }
}

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct User {
        pub id: i64 => vld::number().int().positive(),
        pub name: String => vld::string().min(1),
        pub email: String => vld::string().email(),
    }
}

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct IdParams {
        pub id: String => vld::string().min(1),
    }
}

doc_schema!(CreateUser, User, IdParams);

#[derive(Clone)]
struct Db {
    users: Cell<Vec<User>>,
}

impl Default for Db {
    fn default() -> Self {
        Self {
            users: Cell::new(Vec::new()),
        }
    }
}

fn build_app() -> App {
    let mut app = App::new();
    app.state(Db::default());

    app.get("/users", list).doc(
        Doc::new().ok_schema(json!({
            "type": "array",
            "items": User::json_schema(),
        })),
    );

    app.post("/users", create)
        .doc(Doc::new().body::<CreateUser>().created::<User>());

    app.get("/users/:id", show)
        .doc(Doc::new().params::<IdParams>().ok::<User>());

    app.install(OpenApi::new("Users API", "1.0").mount("/docs"));
    app
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = build_app();
    tracing::info!("API http://127.0.0.1:3000  docs http://127.0.0.1:3000/docs");
    app.listen(3000).await
}

async fn list(req: Request) -> Json<Vec<User>> {
    let db = req.state::<Db>();
    Json(db.users.get())
}

async fn create(mut req: Request) -> std::result::Result<(u16, Json<User>), ValidationError> {
    let body: CreateUser = req.validate().await?;
    let db = req.state::<Db>();
    let mut created = None;
    db.users.update(|users| {
        let mut next = users.clone();
        let id = next.last().map(|u| u.id + 1).unwrap_or(1);
        let user = User {
            id,
            name: body.name.clone(),
            email: body.email.clone(),
        };
        next.push(user.clone());
        created = Some(user);
        next
    });
    Ok((201, Json(created.expect("user"))))
}

async fn show(req: Request) -> Response {
    let params: IdParams = match req.validate_params() {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let id: i64 = params.id.parse().unwrap_or(0);
    let db = req.state::<Db>();
    match db.users.get().into_iter().find(|u| u.id == id) {
        Some(u) => Json(u).into_response(),
        None => Response::text("Not Found").status(404),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use sova::undocumented;

    #[test]
    fn every_route_documented() {
        let app = build_app();
        assert_eq!(undocumented(&app), Vec::<String>::new());
    }

    #[tokio::test]
    async fn create_and_list() {
        let server = build_app().build().unwrap();
        let res = server
            .handle(
                Request::builder()
                    .method(Method::POST)
                    .path("/users")
                    .header("content-type", "application/json")
                    .body(r#"{"name":"Alex","email":"a@b.co"}"#)
                    .build(),
            )
            .await;
        assert_eq!(res.status_code().as_u16(), 201);

        let list = server
            .handle_request(Method::GET, "/users", "")
            .await;
        assert_eq!(list.status_code().as_u16(), 200);

        let spec = server
            .handle_request(Method::GET, "/docs/openapi.json", "")
            .await;
        assert_eq!(spec.status_code().as_u16(), 200);
        let v: serde_json::Value =
            serde_json::from_slice(spec.body_bytes().unwrap()).unwrap();
        assert!(v["paths"].get("/users").is_some());
    }
}
