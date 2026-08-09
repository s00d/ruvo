mod dto;
mod handlers;
mod routes;

use sova::App;

pub fn register(app: &mut App) {
    app.mount("/posts", routes::routes());
}

// entity: crate::entities::post
