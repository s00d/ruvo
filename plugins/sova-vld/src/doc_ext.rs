//! Sugar: `Doc::new().body::<T>()` for vld `schema!` types (feature `openapi`).
//!
//! Prefer route-level [`.validate_body::<T>()`](crate::ValidateRouteExt) which also
//! fills [`sova_openapi::OpenApiValidate`]. These `Doc` helpers remain for response
//! schemas (`ok` / `created`) and manual docs.

use sova_openapi::Doc;

/// Types that expose `json_schema()` (vld `schema!` with vld's `openapi` feature).
pub trait VldDocSchema {
    fn json_schema() -> serde_json::Value;
}

/// Implement [`VldDocSchema`] for vld `schema!` structs.
#[macro_export]
macro_rules! doc_schema {
    ($($t:ty),+ $(,)?) => {
        $(
            impl $crate::VldDocSchema for $t {
                fn json_schema() -> serde_json::Value {
                    <$t>::json_schema()
                }
            }
        )+
    };
}

pub trait DocVldExt {
    /// Prefer `.validate_body::<T>()` on the route for request bodies.
    fn body<T: VldDocSchema>(self) -> Self;
    /// Prefer `.validate_query::<T>()` on the route.
    fn query<T: VldDocSchema>(self) -> Self;
    /// Prefer `.validate_params::<T>()` on the route.
    fn params<T: VldDocSchema>(self) -> Self;
    fn ok<T: VldDocSchema>(self) -> Self;
    fn created<T: VldDocSchema>(self) -> Self;
}

impl DocVldExt for Doc {
    fn body<T: VldDocSchema>(self) -> Self {
        self.body_schema(T::json_schema())
    }
    fn query<T: VldDocSchema>(self) -> Self {
        self.query_schema(T::json_schema())
    }
    fn params<T: VldDocSchema>(self) -> Self {
        self.params_schema(T::json_schema())
    }
    fn ok<T: VldDocSchema>(self) -> Self {
        self.ok_schema(T::json_schema())
    }
    fn created<T: VldDocSchema>(self) -> Self {
        self.created_schema(T::json_schema())
    }
}
