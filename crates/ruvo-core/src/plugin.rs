use crate::app::App;

/// Single extension trait for the framework.
///
/// Prefer `app.install(|app| { ... })` or `app.install(Cors::new())` —
/// users rarely need to name this trait.
pub trait Plugin {
    /// Stable plugin identifier used for dependency checks.
    fn id(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Required plugin ids that must be installed beforehand.
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }

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
