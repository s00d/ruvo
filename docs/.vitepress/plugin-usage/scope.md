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
