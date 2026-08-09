mod api;
mod auth_routes;
mod cabinet;
mod fetch_demo;
mod home;
pub mod notes;
mod upload;
mod ws;

use sova::{preload_unread, App, Fortify, Next, Request, Router};

pub fn register(app: &mut App) {
    home::register(app);
    auth_routes::register(app);

    let mut cabinet = Router::new();
    cabinet.use_middleware(Fortify::guard());
    cabinet.use_middleware(|mut req: Request, next: Next| async move {
        preload_unread(&mut req).await;
        next(req).await
    });
    cabinet::mount(&mut cabinet);
    notes::mount(&mut cabinet);
    upload::mount(&mut cabinet);
    fetch_demo::mount(&mut cabinet);
    ws::mount(&mut cabinet);
    app.mount("/cabinet", cabinet);

    api::register(app);
}
