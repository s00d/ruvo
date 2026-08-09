---
title: Plugin SDK
editLink: false
---

# Plugin SDK

![Plugin SDK](/banners/plugin-sdk.svg)

Write `sova-*` plugins against `sova_core::extend` and the [`Plugin`](/api/plugin-sdk/plugin-trait) trait.
App users: use the [Plugins](/plugins/) catalog instead.

How pages are built: guides in [`plugin-sdk-guides`](https://github.com/s00d/sova/tree/master/docs/.vitepress/plugin-sdk-guides) → `sova-docs-gen` → `docs/api/plugin-sdk*`.

## Pages

<!-- generated:plugin-sdk-toc -->
| Page | Summary |
|------|---------|
| [`Overview`](/api/plugin-sdk/overview) | Mental model, import surfaces, and install checklist for plugin authors. |
| [`Plugin trait`](/api/plugin-sdk/plugin-trait) | id, meta, requires, install, closure plugins, and SDK versioning. |
| [`Middleware`](/api/plugin-sdk/middleware) | named, with_leaked, with_state, MwEntry — when to use each. |
| [`State & dependencies`](/api/plugin-sdk/state) | app.state, markers, Needs, hard requires vs soft has_plugin / try_state. |
| [`Config`](/api/plugin-sdk/config) | Toml unset-fill, env precedence, parse_duration / parse_bytes, features. |
| [`Lifecycle & services`](/api/plugin-sdk/lifecycle) | on_startup / on_shutdown, pool pattern, BackgroundService, CLI mode. |
| [`Checks & CLI`](/api/plugin-sdk/checks-cli) | register_check vs register_audit, probes, register_cli commands. |
| [`Routes & introspection`](/api/plugin-sdk/routes) | Plugin routes, path helpers, RouteValue / MetaMap, match captures. |
| [`Extractors & Problem+`](/api/plugin-sdk/extractors) | Path/Json/State handlers, EventBus, API problem+json errors. |
| [`Events`](/api/plugin-sdk/events) | Typed EventBus listen/dispatch for in-process hooks. |
| [`HTML & log hooks`](/api/plugin-sdk/html-hooks) | HTML inject, logger_skip_path, add_log_event_hook for DevTools-style sinks. |
| [`Errors`](/api/plugin-sdk/errors) | Startup Err vs panic, ErrorResponse, soft degradation. |
| [`Recipes`](/api/plugin-sdk/recipes) | Patterns copied from in-tree plugins (cookies→csrf, pools, tasks, store). |
| [`extend API`](/api/plugin-sdk/extend-api) | Symbol table for sova_core::extend — what it is and who uses it. |
| [`Testing`](/api/plugin-sdk/testing) | TestClient, tracing hooks, feature matrix tips. |
<!-- /generated:plugin-sdk-toc -->

## Quick links

- Start: [Overview](/api/plugin-sdk/overview) · [Plugin trait](/api/plugin-sdk/plugin-trait)
- Cookbook: [Middleware](/api/plugin-sdk/middleware) · [State](/api/plugin-sdk/state) · [Recipes](/api/plugin-sdk/recipes)
- Reference: [extend API](/api/plugin-sdk/extend-api) · [Testing](/api/plugin-sdk/testing)
- Scaffold: `cargo sovax generate plugin <name>`
