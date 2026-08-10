//! Plugin SDK version gate and metadata.

use sova_core::{App, Plugin, PluginMeta, PluginSdkVersion, PLUGIN_SDK_VERSION};

struct OkPlugin;

impl Plugin for OkPlugin {
    fn id(&self) -> &'static str {
        "ok-plugin"
    }

    fn meta(&self) -> PluginMeta {
        PluginMeta::new("Ok")
            .description("matches current SDK")
            .sdk(PLUGIN_SDK_VERSION)
    }

    fn install(self, _app: &mut App) {}
}

struct OldMajorPlugin;

impl Plugin for OldMajorPlugin {
    fn id(&self) -> &'static str {
        "old-major"
    }

    fn meta(&self) -> PluginMeta {
        PluginMeta::new("Old Major").sdk(PluginSdkVersion::new(0, 9, 0))
    }

    fn install(self, _app: &mut App) {}
}

struct NewerMinorPlugin;

impl Plugin for NewerMinorPlugin {
    fn id(&self) -> &'static str {
        "newer-minor"
    }

    fn meta(&self) -> PluginMeta {
        PluginMeta::new("Newer").sdk(PluginSdkVersion::new(
            PLUGIN_SDK_VERSION.major,
            PLUGIN_SDK_VERSION.minor + 1,
            0,
        ))
    }

    fn install(self, _app: &mut App) {}
}

struct OlderMinorPlugin;

impl Plugin for OlderMinorPlugin {
    fn id(&self) -> &'static str {
        "older-minor"
    }

    fn meta(&self) -> PluginMeta {
        // Same major; older than current → warn only (build still ok).
        PluginMeta::new("Older").sdk(PluginSdkVersion::new(PLUGIN_SDK_VERSION.major, 0, 0))
    }

    fn install(self, _app: &mut App) {}
}

#[test]
fn matching_sdk_builds() {
    let mut app = App::new();
    app.install(OkPlugin);
    assert!(app.build().is_ok());
    let metas = app.installed_plugin_meta();
    assert_eq!(metas.len(), 1);
    assert_eq!(metas[0].id, "ok-plugin");
    assert_eq!(metas[0].meta.description, "matches current SDK");
}

#[test]
fn major_mismatch_fails_build() {
    let mut app = App::new();
    app.install(OldMajorPlugin);
    let err = match app.build() {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected SDK major mismatch"),
    };
    assert!(err.contains("old-major"), "{err}");
    assert!(err.contains("major"), "{err}");
}

#[test]
fn plugin_newer_than_core_fails_build() {
    let mut app = App::new();
    app.install(NewerMinorPlugin);
    let err = match app.build() {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected plugin-newer SDK error"),
    };
    assert!(err.contains("newer-minor"), "{err}");
}

#[test]
fn older_same_major_builds() {
    // Only meaningful when current minor/patch > 0.0; otherwise skip semantics.
    if PLUGIN_SDK_VERSION.minor == 0 && PLUGIN_SDK_VERSION.patch == 0 {
        // Still builds: equal to "1.0.0" when current is 1.0.0 — use a synthetic older via
        // forcing patch path: if we're exactly 1.0.0, older same-major isn't representable
        // without going to 0.x (major fail). Treat as no-op success for matching major.
        let mut app = App::new();
        app.install(OkPlugin);
        assert!(app.build().is_ok());
        return;
    }
    let mut app = App::new();
    app.install(OlderMinorPlugin);
    assert!(app.build().is_ok());
}
