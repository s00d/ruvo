---
title: static
editLink: false
---

# `static`

**Serve files from a directory under a mount path**

| | |
|--|--|
| Crate | [`sova-static`](https://docs.rs/sova-static/0.1.1) `0.1.1` |
| Plugin id | `static` |
| Category | HTTP |

## Install

```bash
cargo add sova --features static-files
```

## Features

| Feature | What you get |
|---------|-------------|
| `static-files` | Serve a directory under a URL mount (`Static`). |

## Overview

**When:** serve `public/` (or any dir) under a mount path. Already on `App::web()` as `/assets`.

**Does:**
- `mount` + `mount/*path` routes
- `max_age`, `immutable`, index files, dotfile guard

### Example

```rust
app.install(Static::new("/assets", "public").max_age(Duration::from_secs(3600)));
```

## Quick start

`App::web()` already mounts `public/` → `/assets`. Extra mount:

```rust
use sova::{App, Static};
use std::time::Duration;

app.install(
    Static::new("/static", "assets")
        .max_age(Duration::from_secs(86_400))
        .immutable(true),
);
```

Dotfiles denied by default. See `examples/web/static_files`.

## Examples

- `examples/web/static_files`

## Related

[`storage`](/plugins/storage) · [`templates`](/plugins/templates) · [`fs`](/plugins/fs)
