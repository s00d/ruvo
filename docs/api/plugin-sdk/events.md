---
title: Events
editLink: false
---

# Events

EventBus listen/dispatch + catalog of first-party plugin events.

> Author guide — edit `docs/.vitepress/plugin-sdk-guides/events.md`, then `pnpm docs:generate`.

In-process typed events (`Event` + `EventBus`) for plugins and apps. Listeners run **synchronously** in registration order inside `dispatch`. For async work, `tokio::spawn` / `TaskBackend::dispatch` from the listener.

```rust
use sova::{Event, EventBus};

#[derive(Clone)]
struct NoteCreated { id: i64 }
impl Event for NoteCreated {
    fn name(&self) -> &'static str { "app.note_created" }
}

let bus = app.events(); // inserts EventBus into app.state on first call
bus.listen::<NoteCreated, _>(|e| {
    tracing::info!(note_id = e.id, "created");
});
// somewhere later:
app.events().dispatch(NoteCreated { id: 1 });
```

Soft-wire pattern in plugins: `client.set_events(app.events())` / hold `Option<EventBus>` and `dispatch` after mutations (mail, fs, auth, …).

DevTools (feature-gated) mirrors many of these into the timeline — see [devtools](/plugins/devtools).

---

## Built-in events (first-party plugins)

Stable string from `Event::name()`. Types live in the plugin crate (re-exported by facade features).

| `name()` | Type | Crate / feature | Payload |
|----------|------|-----------------|---------|
| `auth.user_registered` | `UserRegistered` | [auth](/plugins/auth) | `user_id`, `email` |
| `auth.user_logged_in` | `UserLoggedIn` | auth | `user_id`, `email` |
| `mail.sent` | `MailSent` | [mail](/plugins/mail) | `to`, `subject` |
| `csrf.mismatch` | `CsrfMismatch` | [csrf](/plugins/csrf) | `method`, `path` |
| `session.regenerated` | `SessionRegenerated` | [session](/plugins/session) | `had_user` |
| `session.logout_all` | `SessionLogoutAll` | session | `user_id`, `count` |
| `rate_limit.exceeded` | `RateLimitExceeded` | [rate-limit](/plugins/rate-limit) | `key`, `limit`, `retry_after` |
| `tasks.dispatched` | `TaskDispatched` | [tasks](/plugins/tasks) | `id`, `name`, `queue` |
| `tasks.failed` | `TaskFailed` | tasks | `id`, `name`, `attempts` |
| `notifications.sent` | `NotificationSent` | [notifications](/plugins/notifications) | `channel`, `event`, `recipients` |
| `passport.api_token_revoked` | `ApiTokenRevoked` | [passport](/plugins/passport) | `user_id`, `token_id` |
| `acme.certificate_issued` | `CertificateIssued` | [acme](/plugins/acme) | `domains`, `not_after_unix` |
| `acme.certificate_renewed` | `CertificateRenewed` | acme | `domains`, `not_after_unix` |
| `acme.failed` | `AcmeFailed` | acme | `domains`, `error` |
| `fs.file_written` | `FileWritten` | [fs](/plugins/fs) | `path` (relative to jail) |
| `fs.file_removed` | `FileRemoved` | fs | `path` |
| `fs.dir_created` | `DirCreated` | fs | `path` |

Listen example against a first-party type:

```rust
use sova::prelude::*;
use sova::{MailSent, UserRegistered};

// after plugins are installed — bus is shared
app.events().listen::<UserRegistered, _>(|e| {
    tracing::info!(id = e.user_id, email = %e.email, "registered");
});
app.events().listen::<MailSent, _>(|e| {
    tracing::debug!(?e.to, subject = %e.subject, "mail accepted");
});
```

Apps may define their own `Event` types (cabinet `NoteCreated`, etc.) — they are **not** in the table above.

---

## Rules for new plugin events

1. One Rust type per event; `name()` uses `plugin.action` snake style (`auth.user_registered`).
2. Payload: cheap `Clone` fields (ids, strings) — no request bodies.
3. Dispatch after the side effect succeeded (or immediately before a terminal error response for security signals like CSRF / rate-limit).
4. Soft-dep on `EventBus` — do not require a separate plugin install.
5. Optionally wire into DevTools `hub` behind a feature flag.

See also: [Extractors & Problem+](/api/plugin-sdk/extractors) · [Lifecycle](/api/plugin-sdk/lifecycle)
