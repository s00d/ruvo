//! Mount the same router under locale prefixes.

use ruvo_core::Router;

/// How to apply locale path prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixMode {
    /// Mount under `/en`, `/de`, … for every locale.
    Prefix,
    /// Default locale has no prefix; others do.
    PrefixExceptDefault,
}

/// Mount `routes` under locale prefixes onto `app_router`.
///
/// `routes` is cloned via re-registration: pass a builder closure instead.
pub fn mount_localized<F>(
    app: &mut Router,
    locales: &[String],
    default: &str,
    mode: PrefixMode,
    mut build: F,
) where
    F: FnMut(&mut Router),
{
    match mode {
        PrefixMode::Prefix => {
            for loc in locales {
                let mut child = Router::new();
                build(&mut child);
                app.mount(&format!("/{loc}"), child);
            }
        }
        PrefixMode::PrefixExceptDefault => {
            build(app);
            for loc in locales {
                if loc == default {
                    continue;
                }
                let mut child = Router::new();
                build(&mut child);
                app.mount(&format!("/{loc}"), child);
            }
        }
    }
}
