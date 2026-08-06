//! Typed route/router/app attributes: [`RouteValue`] + [`MetaMap`].

use crate::state::StateMap;
use http::Method;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Value attached via [`crate::Router::with`] (route, router, or app scope).
pub trait RouteValue: Any + Send + Sync + 'static {
    /// Startup validation; default is a no-op.
    fn check(&self, _ctx: &BuildCtx<'_>) -> Result<(), String> {
        Ok(())
    }

    /// Label for `explain` / CLI `check`.
    fn label(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }
}

/// Context passed to [`RouteValue::check`] during [`crate::App::build`].
pub struct BuildCtx<'a> {
    pub state: &'a StateMap,
    pub installed_plugins: &'a HashSet<&'static str>,
    pub route_path: &'a str,
    pub route_method: Option<&'a Method>,
}

type CheckFn = Arc<dyn Fn(&BuildCtx<'_>) -> Result<(), String> + Send + Sync>;

/// Typed metadata bag for routes/routers (one value per `TypeId`; last insert wins).
#[derive(Default, Clone)]
pub struct MetaMap {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    checkers: HashMap<TypeId, CheckFn>,
    labels: HashMap<TypeId, String>,
}

impl MetaMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: RouteValue>(&mut self, value: T) {
        let id = TypeId::of::<T>();
        let label = value.label().into_owned();
        let arc = Arc::new(value);
        let for_check = Arc::clone(&arc);
        self.map.insert(id, arc);
        self.checkers.insert(
            id,
            Arc::new(move |ctx| RouteValue::check(for_check.as_ref(), ctx)),
        );
        self.labels.insert(id, label);
    }

    pub fn get<T: RouteValue>(&self) -> Option<Arc<T>> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.clone().downcast::<T>().ok())
    }

    /// Merge another map; later values overwrite the same `TypeId`.
    pub fn extend(&mut self, other: MetaMap) {
        for (id, v) in other.map {
            self.map.insert(id, v);
        }
        for (id, c) in other.checkers {
            self.checkers.insert(id, c);
        }
        for (id, l) in other.labels {
            self.labels.insert(id, l);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn check_all(&self, ctx: &BuildCtx<'_>) -> Result<(), String> {
        for check in self.checkers.values() {
            check(ctx)?;
        }
        Ok(())
    }

    /// Labels for introspection (stable order by label string).
    pub fn labels(&self) -> Vec<&str> {
        let mut v: Vec<_> = self.labels.values().map(|s| s.as_str()).collect();
        v.sort_unstable();
        v
    }
}

/// Declares that application state must contain `T` before serving.
pub struct Needs<T>(std::marker::PhantomData<fn() -> T>);

impl<T: Send + Sync + 'static> Needs<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: Send + Sync + 'static> Default for Needs<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + Sync + 'static> RouteValue for Needs<T> {
    fn check(&self, ctx: &BuildCtx<'_>) -> Result<(), String> {
        if ctx.state.get::<T>().is_some() {
            Ok(())
        } else {
            Err(format!(
                "route {} needs state `{}` (missing in App::state)",
                ctx.route_path,
                std::any::type_name::<T>()
            ))
        }
    }

    fn label(&self) -> Cow<'static, str> {
        Cow::Owned(format!("Needs<{}>", std::any::type_name::<T>()))
    }
}
