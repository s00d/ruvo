use crate::app::App;

/// Single extension trait for the framework.
///
/// Prefer `app.install(|app| { ... })` or `app.install(Cors::new())` —
/// users rarely need to name this trait.
pub trait Plugin {
    fn install(self, app: &mut App);
}

impl<F> Plugin for F
where
    F: FnOnce(&mut App),
{
    fn install(self, app: &mut App) {
        self(app);
    }
}
