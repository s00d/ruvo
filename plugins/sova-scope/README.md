# sova-scope

Scoped RBAC for [Sova](https://github.com/s00d/sova): membership in a scope (workspace, team, org), declarative role matrix, optional ownership hooks, invitations, membership cache, and role audit.

Library-first — no HTTP routes. Apps register a matrix and call `ScopeExt` from handlers.

Docs: [plugin page](https://sova.rs/plugins/scope) (guides under `docs/.vitepress/plugin-guides/scope.md` + `plugin-usage/scope.md`).

```rust
app.install(
    ScopeAuth::new()
        .configure(|b| {
            b.allow_all("workspace", "admin")
                .allow("workspace", "editor", Ability::View, ResourcePattern::All, Condition::Always)
                .allow(
                    "workspace",
                    "editor",
                    Ability::Update,
                    ResourcePattern::Kind("page"),
                    Condition::Owner,
                );
        })
        .owner("page", |db, id| async move { /* Some(owner_user_id) */ Ok(None) }),
);
```

## Cache + audit

`AuthorizeEngine` caches positive membership lookups. Pass `MutateOpts { actor_id, cache }` into `MembershipStore::*_with` / `InviteStore::*_with` so invite accept and role changes invalidate the cache and write `scope_membership_audit`.
