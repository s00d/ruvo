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
