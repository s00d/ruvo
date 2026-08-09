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

**API errors:** `App::api()` maps framework `Error` to `application/problem+json`. `App::web()` negotiates via `Accept`: prefer `text/html` → minimal HTML error page; `application/problem+json` / `application/json` → Problem Details; otherwise `text/plain`. Router 404/405 use the same Accept-aware builder. Validation (`sova-vld`) uses Problem Details with an `errors` array.
