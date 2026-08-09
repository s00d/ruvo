[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-docs-gen

Generate VitePress plugin catalog + pages from Rust sources.

```bash
pnpm docs:generate
# or
cargo run -p sova-docs-gen
cargo run -p sova-docs-gen -- --check
```

Not published (`publish = false`).

## Page recipe (per plugin)

| Layer | Source | Becomes |
|-------|--------|---------|
| Summary | `Plugin::meta().description` (else Cargo `description`) | lead + catalog cell |
| Install / features | `crates/sova/Cargo.toml` + `doc_features.rs` | `## Install` / `## Features` |
| Overview | `docs/.vitepress/plugin-guides/<slug>.md` **or** crate `//!` | `## Overview` |
| Quick start | `docs/.vitepress/plugin-usage/<slug>.md` | `## Quick start` |
| Examples / related | maps in `sova-docs-gen` | `## Examples` / `## Related` |
| Nav | categories in gen | grouped sidebar |

Also patches the catalog table in `docs/plugins/index.md` (`<!-- generated:plugins-table -->`) and writes `docs/api/plugin-sdk.md`.

## Authoring

1. **Guide** (`plugin-guides/`) — what / when / bullets / short example / config / pitfalls. Prefer this over long `//!` for VitePress.
2. **Usage** (`plugin-usage/`) — install wiring + handler snippets (code-first).
3. **Features** — `/// Feature \`name\`: …` in `crates/sova/src/doc_features.rs`.
4. **Crate rustdoc** — keep `//!` useful on docs.rs; guides win on the site when present.
5. Never hand-edit `docs/plugins/<slug>.md` — regenerate.

## License

MIT — see [LICENSE](LICENSE).
