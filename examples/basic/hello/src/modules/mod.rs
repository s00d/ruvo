use sova::App;

pub mod auth;
pub mod blog;

pub fn register(app: &mut App) {
    app.mount("/auth", auth::routes());
    app.mount("/blog", blog::routes());
}
