---
title: Plugin trait
editLink: false
---

# Plugin trait

id, meta, requires, install, closure plugins, and SDK versioning.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/plugin-trait.md`, then `pnpm docs:generate`.

Implement [`Plugin`](https://docs.rs/sova-core/latest/sova_core/trait.Plugin.html) on a struct (or use a closure). Core methods:

| Method | Default | Use |
|--------|---------|-----|
| `id()` | `type_name::<Self>()` | **Always override** with a short stable string (`"session"`, `"cookies"`) |
| `requires()` | `&[]` | Hard deps — missing ids fail at `App::build` |
| `meta()` | `PluginMeta::for_id(id)` | Display name, description, crate version, SDK |
| `install(self, &mut App)` | — | Side effects only; consume `self` |

### Identity

```rust
fn id(&self) -> &'static str {
    "hello-header"
}
```

Ids feed `has_plugin`, `requires`, CLI `plugins`, and docs. Prefer slug-like constants over Rust paths.

### Metadata

```rust
fn meta(&self) -> PluginMeta {
    PluginMeta::new("Hello Header")
        .description("Adds an X-Hello response header")
        .version(env!("CARGO_PKG_VERSION"))
        // .author("…")
        // .sdk(PluginSdkVersion::new(1, 0, 0)) // default = PLUGIN_SDK_VERSION
}
```

`description` is scraped by `sova-docs-gen` into the [Plugins](/plugins/) catalog when present.

### Hard dependencies

```rust
fn requires(&self) -> &'static [&'static str] {
    &["session"] // csrf, fortify-style
}
```

Install order still matters for soft logic, but `requires` guarantees the dep was installed **before** this plugin.

### Closure plugins

No named type — fine for app-local wiring:

```rust
app.install(|app: &mut App| {
    app.get("/healthz", || async { Response::text("ok") });
});
```

Closure plugins get a synthetic id from `type_name` — do not use them when other plugins must `requires` you.

### SDK versioning

[`PLUGIN_SDK_VERSION`](https://docs.rs/sova-core/latest/sova_core/constant.PLUGIN_SDK_VERSION.html) versions the **author-facing** surface (independent of crate semver).

On `install`, core compares `meta().sdk` to running core:

| Situation | Result |
|-----------|--------|
| Different **major** | Hard error at build |
| Plugin **newer** than core (same major) | Hard error |
| Core **newer** than plugin (same major) | `tracing` warning |
| Exact match | OK |

Bump major only when breaking plugin APIs. Declare older SDK only if you intentionally target an older surface via `.sdk(…)`.

Helpers: `check_plugin_sdk`, `PluginSdkVersion`, `SdkCompat` (also under `extend`).

### Minimal complete example

```rust
use sova_core::extend::with_leaked;
use sova_core::{App, Plugin, PluginMeta, Request, Response};

struct HelloHeader;

impl Plugin for HelloHeader {
    fn id(&self) -> &'static str {
        "hello-header"
    }

    fn meta(&self) -> PluginMeta {
        PluginMeta::new("Hello Header")
            .description("Adds an X-Hello response header")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        app.use_middleware(with_leaked((), |_s, req, next| async move {
            let mut res = next(req).await;
            res = res.header("x-hello", "sova");
            res
        }));
    }
}
```

See also rustdoc on [`plugin.rs`](https://docs.rs/sova-core/latest/sova_core/trait.Plugin.html) and [Recipes](/plugin-sdk/recipes).

## From `sova-core` rustdoc

Plugin extension trait and SDK metadata.

 Full VitePress guide: <https://s00d.github.io/sova/plugin-sdk/>
 (source: `docs/.vitepress/plugin-sdk-guides/`).

 # Writing a plugin

 A plugin is any type that implements [`Plugin`]. On install it typically:

 1. Registers middleware via [`crate::App::use_middleware`] / [`crate::extend::with_leaked`]
 2. Inserts shared state with [`crate::App::state`]
 3. Adds routes (`get` / `post` / …)
 4. Optionally registers lifecycle hooks, CLI commands, or checks

 ## Identity and dependencies

 Override [`Plugin::id`] with a short stable string (`"cookies"`, `"session"`).
 Use [`Plugin::requires`] so dependents fail at [`crate::App::build`] if a
 dependency was not installed first. Prefer short ids over `type_name`.

 ## SDK versioning

 [`PLUGIN_SDK_VERSION`] is the plugin-author surface version (independent of
 the crate semver). Declare the version your plugin was built against via
 [`PluginMeta::sdk`] (default = current). Compatibility on install:

 - **different major** → hard error at build
 - **plugin newer** than core (same major) → hard error
 - **core newer** than plugin (same major) → `tracing` warning

 Bump `PLUGIN_SDK_VERSION` major only when the author-facing API breaks.

 ## Metadata

 [`Plugin::meta`] returns human-readable info for CLI (`plugins`) and docs.

 Scaffold: `cargo sovax generate plugin <name>`.
