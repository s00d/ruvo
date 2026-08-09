**When:** register / login / verify / reset / 2FA / profile / roles (Fortify-style).

**Does:**
- Web forms + JSON API mounts
- Feature flags: Registration, ResetPasswords, EmailVerification, TwoFactor, …
- `Fortify::guard()` for protected routers
- `req.login_user` / `req.logout_user`
- Optional mail + activity

### Example

```rust
app.install(Db::from_env().migrations::<AuthMigrator>());
app.install(Mail::from_env()); // only if Reset / Verify
app.install(
  Fortify::new()
    .features([AuthFeature::Registration, AuthFeature::ResetPasswords])
    .home("/cabinet"),
);
cabinet.use_middleware(Fortify::guard());
```

### Notes
- Needs **db + session**
- Add **mail** only for email-backed features (`auth-mail`)

### Config

```bash
FORTIFY_SECRET=…     # or APP_KEY — token signing
PUBLIC_URL=https://… # links in verify/reset mail
APP_NAME=MyApp
```

No `[auth]` TOML section — features/paths are builder (`Fortify::new().features([...]).home(...)`).

### Policies

Resource-level authorize (owner / role / permission) without a registry middleware:

```rust
use sova::{Ability, AuthExt, Policy};

#[derive(Default)]
struct NotePolicy;
impl Policy<Note> for NotePolicy {
    fn view(&self, user: &CurrentUser, n: &Note) -> bool {
        user.id == n.user_id || user.has_role("admin")
    }
    fn update(&self, user: &CurrentUser, n: &Note) -> bool { self.view(user, n) }
    fn delete(&self, user: &CurrentUser, n: &Note) -> bool { self.view(user, n) }
}

// after loading the note:
req.authorize::<NotePolicy, _>(Ability::Delete, &note)?;
```

Route-level `Fortify::permission("…")` remains for abilities without a loaded model.
