mod api;
mod auth_routes;
mod cabinet;
mod fetch_demo;
mod home;
mod notes;
mod upload;
mod ws;

use ruvo::{App, Fortify, Router};

pub fn register(app: &mut App) {
    home::register(app);
    auth_routes::register(app);

    let mut cabinet = Router::new();
    cabinet.use_middleware(Fortify::guard());
    cabinet::mount(&mut cabinet);
    notes::mount(&mut cabinet);
    upload::mount(&mut cabinet);
    fetch_demo::mount(&mut cabinet);
    ws::mount(&mut cabinet);
    app.mount("/cabinet", cabinet);

    api::register(app);
}
