[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-docs-gen

Generate VitePress plugin catalog + Plugin SDK pages from Rust sources and guide markdown.

```bash
pnpm docs:generate
# or
cargo run -p sova-docs-gen
cargo run -p sova-docs-gen -- --check
```

Not published (`publish = false`).

## Plugins — page recipe

| Layer | Source | Becomes |
|-------|--------|---------|
| Summary | `Plugin::meta().description` (else Cargo `description`) | lead + catalog cell |
| Install / features | `crates/sova/Cargo.toml` + `doc_features.rs` | `## Install` / `## Features` |
| Overview | `docs/.vitepress/plugin-guides/<slug>.md` **or** crate `//!` | `## Overview` |
| Quick start | `docs/.vitepress/plugin-usage/<slug>.md` | `## Quick start` |
| Examples / related | maps in `sova-docs-gen` | `## Examples` / `## Related` |
| Nav | categories in gen | grouped sidebar |

Also patches the catalog table in `docs/plugins/index.md` (`<!-- generated:plugins-table -->`).

**Never hand-edit** `docs/plugins/<slug>.md` — regenerate.

## Plugin SDK — page recipe

| Layer | Source | Becomes |
|-------|--------|---------|
| Index | ordered list in `sova-docs-gen` | `docs/api/plugin-sdk.md` → `/api/plugin-sdk` |
| Body | `docs/.vitepress/plugin-sdk-guides/<slug>.md` | `docs/api/plugin-sdk/<slug>.md` |
| Nav | groups Start / Cookbook / Patterns / Reference | `plugin-sdk-nav.generated.ts` |
| Trait rustdoc | `crates/sova-core/src/plugin.rs` `//!` | appended on Plugin trait page |

**Author guides** under `plugin-sdk-guides/`; do not hand-edit generated `docs/api/plugin-sdk*`.

Page order / titles live in `plugin_sdk_pages()` inside `src/main.rs`.

## Authoring plugins catalog

1. **Guide** (`plugin-guides/`) — what / when / bullets / short example / config / pitfalls.
2. **Usage** (`plugin-usage/`) — install wiring + handler snippets (code-first).
3. **Features** — `/// Feature \`name\`: …` in `crates/sova/src/doc_features.rs`.
4. **Crate rustdoc** — keep `//!` useful on docs.rs; guides win on the site when present.

## Authoring Plugin SDK

1. Edit the matching file in `docs/.vitepress/plugin-sdk-guides/`.
2. To add a page: append an entry in `plugin_sdk_pages()` and create the guide file.
3. Run `pnpm docs:generate`.
4. Keep `plugin.rs` rustdoc short; long narrative belongs in VitePress.

## License

MIT — see [LICENSE](LICENSE).
