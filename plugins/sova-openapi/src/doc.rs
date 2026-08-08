use serde_json::Value;

/// Per-route OpenAPI annotations stored in route meta.
#[derive(Debug, Clone)]
pub struct Doc {
    pub(crate) skip: bool,
    pub(crate) body_schema: Option<Value>,
    pub(crate) query_schema: Option<Value>,
    pub(crate) params_schema: Option<Value>,
    pub(crate) responses: Vec<(u16, Value)>,
}

impl Doc {
    pub fn new() -> Self {
        Self {
            skip: false,
            body_schema: None,
            query_schema: None,
            params_schema: None,
            responses: Vec::new(),
        }
    }

    /// Mark the route as intentionally undocumented.
    pub fn skip() -> Self {
        Self {
            skip: true,
            ..Self::new()
        }
    }

    pub fn is_skip(&self) -> bool {
        self.skip
    }

    pub fn body_schema(mut self, schema: Value) -> Self {
        self.body_schema = Some(schema);
        self
    }

    pub fn query_schema(mut self, schema: Value) -> Self {
        self.query_schema = Some(schema);
        self
    }

    pub fn params_schema(mut self, schema: Value) -> Self {
        self.params_schema = Some(schema);
        self
    }

    pub fn ok_schema(mut self, schema: Value) -> Self {
        self.responses.push((200, schema));
        self
    }

    pub fn created_schema(mut self, schema: Value) -> Self {
        self.responses.push((201, schema));
        self
    }

    pub fn response(mut self, status: u16, schema: Value) -> Self {
        self.responses.push((status, schema));
        self
    }
}

impl Default for Doc {
    fn default() -> Self {
        Self::new()
    }
}
