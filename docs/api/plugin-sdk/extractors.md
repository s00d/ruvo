---
title: Extractors & Problem+
editLink: false
---

# Extractors & Problem+

Path/Json/State handlers, EventBus, API problem+json errors.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/extractors.md`, then `pnpm docs:generate`.

**Handlers:** typed extractors (`Path`, `Query`, `Json`, `Form`, `State`, `Extension`) via `sova::extract` — separate `IntoHandler` markers so classic `async fn(req: Request)` stays valid.

```rust
use sova::extract::{Json, Path, State};

#[derive(serde::Deserialize)]
struct Id { id: String }

async fn show(Path(Id { id }): Path<Id>, State(db): State<Db>) -> Response {
    /* … */
}
```

`vld` schemas use `VldParse` (not serde `Deserialize`) — keep `req.validate()` / route validate for those types; use plain serde structs with `extract::Json`.

**Events:** `app.events()` returns a shared [`EventBus`](https://docs.rs/sova-core). Sync listeners run in the dispatching task:

```rust
let bus = app.events();
bus.listen::<NoteCreated, _>(|e| tracing::info!(?e.note_id, "created"));
// later:
bus.dispatch(NoteCreated { note_id: 1, user_id: 2 });
```

**API errors:** `App::api()` installs an `error_handler` that maps framework `Error` to `application/problem+json`. Validation (`sova-vld`) uses the same Problem Details shape with an `errors` array.
