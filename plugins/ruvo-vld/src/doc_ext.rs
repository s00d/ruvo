//! Sugar: `Doc::new().body::<T>()` for vld `schema!` types (feature `openapi`).

use ruvo_openapi::Doc;

/// Types that expose `json_schema()` (vld `schema!` with vld's `openapi` feature).
///
/// Implement with [`doc_schema!`]:
/// ```ignore
/// vld::schema! { pub struct User { ... } }
/// ruvo_vld::doc_schema!(User);
/// ```
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
    fn body<T: VldDocSchema>(self) -> Self;
    fn query<T: VldDocSchema>(self) -> Self;
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
