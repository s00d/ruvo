//! Named notification channels with optional ACL.

/// Delivery channel config (code-defined, not a DB table).
#[derive(Clone, Debug)]
pub struct Channel {
    pub slug: String,
    /// Permission required to publish via HTTP broadcast (None = open to callers who hit the route guard).
    pub publish: Option<String>,
    /// Permission required to subscribe / list this channel (None = any authenticated recipient).
    pub subscribe: Option<String>,
}

impl Channel {
    pub fn new(slug: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            publish: None,
            subscribe: None,
        }
    }

    pub fn publish(mut self, permission: impl Into<String>) -> Self {
        self.publish = Some(permission.into());
        self
    }

    pub fn subscribe(mut self, permission: impl Into<String>) -> Self {
        self.subscribe = Some(permission.into());
        self
    }
}

/// How a notification is delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    Database,
    #[cfg(feature = "ws")]
    Ws,
    #[cfg(feature = "mail")]
    Mail,
}
