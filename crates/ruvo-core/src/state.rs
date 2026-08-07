use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Typed bag keyed by [`TypeId`] (route meta, shared app state).
///
/// Inserting the same `T` twice keeps the **last** value. Different types never
/// conflict; call order across types does not matter.
#[derive(Default, Clone)]
pub struct TypeMap {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

/// Shared application state: `app.state(db)` / `req.state::<Database>()`.
pub type StateMap = TypeMap;

impl TypeMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.map.insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.clone().downcast::<T>().ok())
    }

    /// Merge another map; later values overwrite the same TypeId.
    pub fn extend(&mut self, other: TypeMap) {
        self.map.extend(other.map);
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub(crate) fn clone_map(&self) -> TypeMap {
        self.clone()
    }
}

/// Per-request typed bag: `req.set(user)` / `req.get::<User>()`.
#[derive(Default)]
pub struct Extensions {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|v| v.downcast_mut::<T>())
    }

    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast::<T>().ok().map(|b| *b))
    }
}

/// Route metadata bag attached to the request after a successful match.
#[derive(Clone)]
pub struct MatchedMeta(pub crate::route_value::MetaMap);

/// Optional slot filled when a route matches — for root middleware that runs
/// before match but needs meta after `next` (e.g. SEO head inject).
#[derive(Clone, Default)]
pub struct MatchedMetaCapture {
    inner: std::sync::Arc<std::sync::Mutex<Option<crate::route_value::MetaMap>>>,
}

impl MatchedMetaCapture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, meta: crate::route_value::MetaMap) {
        *self.inner.lock().unwrap() = Some(meta);
    }

    pub fn get(&self) -> Option<crate::route_value::MetaMap> {
        self.inner.lock().unwrap().clone()
    }
}
