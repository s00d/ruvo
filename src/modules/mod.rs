pub mod post;
use sova::App;

pub fn register(app: &mut App) {
    post::register(app);
}
