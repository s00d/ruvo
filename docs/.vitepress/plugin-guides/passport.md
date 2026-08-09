**When:** JWT access/refresh, personal access tokens, OAuth login.

**Does:**
- Users + refresh tokens + PAT (`svpat_…`)
- `JwtAuth::guard` (Bearer JWT or PAT)
- OAuth: GitHub / Google / Apple / Custom

### Example

```rust
app.install(Passport::new().jwt(/* … */));
api.use_middleware(JwtAuth::guard());
```

### Notes
- OAuth env: `{NAME}_CLIENT_ID` / `{NAME}_CLIENT_SECRET`
