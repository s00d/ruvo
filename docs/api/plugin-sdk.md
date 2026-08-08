---
title: Plugin SDK
editLink: false
---

# Plugin SDK

![Plugin SDK](/banners/plugin-sdk.svg?v=11)

> Auto-generated from `crates/sova-core/src/plugin.rs`. For writing plugins — app usage is under [Plugins](/plugins/).

Plugin extension trait and SDK metadata.

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

 # Examples

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

 let mut app = App::new();
 app.install(HelloHeader);
 ```

 Closure plugins work without a named type:

```rust
 app.install(|app: &mut App| {
     app.get("/healthz", || async { Response::text("ok") });
 });
 ```

 Scaffold a new crate with `cargo sovax generate plugin <name>`.
