**When:** multi-tenant / workspace RBAC (membership in a scope, role matrix, optional resource ownership).

**Does:**
- Library-first — no HTTP routes; apps call `ScopeExt` / `AuthorizeEngine`
- Tables via `ScopeMigrator`: memberships, optional invitations (`invites` feature), membership audit
- Declarative `MatrixBuilder` + `.owner(kind, resolver)` hooks
- In-process `MembershipCache` (positive hits); invalidate on mutate / invite accept
- `AuditStore` for who gained / changed / lost a role

### Example

```rust
app.install(Db::from_env().migrations::<ScopeMigrator>());
app.install(
    ScopeAuth::new()
        .configure(|b| {
            b.allow_all("workspace", "owner")
                .allow_all("workspace", "admin")
                .allow("workspace", "editor", Ability::View, ResourcePattern::All, Condition::Always)
                .allow(
                    "workspace",
                    "editor",
                    Ability::Update,
                    ResourcePattern::Kind("page"),
                    Condition::Owner,
                )
                .allow("workspace", "viewer", Ability::View, ResourcePattern::All, Condition::Always);
        })
        .owner("page", |db, page_id| async move {
            // return Some(user_id) if that user owns the resource
            Ok(None)
        }),
);

// In a handler:
let caps = req
    .scope_authorize(ScopeRef::new("workspace", ws_id), uid, Ability::Update, Some(ResourceRef::new("page", page_id)))
    .await?;
```

### Mutations + cache

```rust
let engine = req.scope_engine()?;
let opts = MutateOpts::default()
    .actor(admin_id)
    .cache(engine.membership_cache());
MembershipStore::add_with(&db, scope, user_id, "editor", opts).await?;
InviteStore::accept_by_token_with(&db, user_id, &email, &token, opts).await?;
```

Without `cache` in `MutateOpts`, engine may serve a stale positive hit until `membership_cache().invalidate(scope, user_id)`.

### Audit

Every `add_with` / `update_role_with` / `remove_with` (and invite accept that adds membership) writes `scope_membership_audit` (`added` / `role_changed` / `removed`). Query with `AuditStore::list_for_scope` / `list_for_user`.

### Notes
- Needs **db**
- Feature `invites` (default on) adds invitation store
- Apps that squash schema (e.g. Atlas) must include `scope_membership_audit` themselves
