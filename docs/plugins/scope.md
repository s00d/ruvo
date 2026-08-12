---
title: scope
editLink: false
---

# `scope`

**Scoped RBAC: membership, role matrix, ownership hooks**

| | |
|--|--|
| Crate | [`sova-scope`](https://docs.rs/sova-scope/0.1.0) `0.1.0` |
| Plugin id | `scope` |
| Category | Other |

## Install

```bash
cargo add sova --features scope
```

## Features

| Feature | What you get |
|---------|-------------|
| `scope` | — |

## Overview

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

## Quick start

Scoped RBAC on top of Db. Install migrator + `ScopeAuth`, then authorize in handlers.

```rust
use sova::prelude::*;
use sova::{
    Ability, Condition, Db, MatrixBuilder, MembershipStore, MutateOpts, Parser, Request,
    ResourcePattern, ResourceRef, ScopeAuth, ScopeExt, ScopeMigrator, ScopeRef, ServerArgs,
};

fn configure_matrix(b: &mut MatrixBuilder) {
    b.allow_all("workspace", "owner")
        .allow_all("workspace", "admin")
        .allow(
            "workspace",
            "editor",
            Ability::View,
            ResourcePattern::All,
            Condition::Always,
        )
        .allow(
            "workspace",
            "editor",
            Ability::Update,
            ResourcePattern::Kind("page"),
            Condition::Owner,
        )
        .deny(
            "workspace",
            "editor",
            Ability::Delete,
            ResourcePattern::All,
            Condition::Always,
        )
        .allow(
            "workspace",
            "viewer",
            Ability::View,
            ResourcePattern::All,
            Condition::Always,
        );
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.init_tracing();

    let mut app = App::api().into_app();
    app.install(Db::from_env().migrations::<ScopeMigrator>());
    app.install(
        ScopeAuth::new()
            .configure(configure_matrix)
            .owner("page", |_db, _id| async move { Ok(None) }),
    );

    app.get("/w/:id/pages/:pid", |req: Request| async move {
        let uid = /* current user */ 1i64;
        let ws: i64 = req.param("id").unwrap().parse().unwrap();
        let pid: i64 = req.param("pid").unwrap().parse().unwrap();
        req.scope_authorize(
            ScopeRef::new("workspace", ws),
            uid,
            Ability::View,
            Some(ResourceRef::new("page", pid)),
        )
        .await?;
        Ok("ok")
    });

    app.run().await
}
```

Invite accept with cache invalidate + audit actor:

```rust
use sova_scope::{InviteStore, MutateOpts, ScopeExt};

let engine = req.scope_engine()?;
let opts = MutateOpts::default().cache(engine.membership_cache());
InviteStore::accept_by_token_with(&db, uid, &email, &token, opts).await?;
```

Role change:

```rust
MembershipStore::update_role_with(
    &db,
    membership_id,
    "admin",
    MutateOpts::default()
        .actor(actor_id)
        .cache(engine.membership_cache()),
)
.await?;
```

See a separate Atlas wiki demo (not in this repo) for a full workspace matrix and owner hooks on `page` / `poll`.
