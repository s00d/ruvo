mod feed;
mod item;
mod submit;

use sova::App;

pub fn register(app: &mut App) {
    feed::register(app);
    submit::register(app);
    item::register(app);
}
