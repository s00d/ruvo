//! Domain entities for the HN demo (stories / votes / comments).
//! Auth users live in Fortify's `auth_users` (see [`sova::AuthMigrator`]).

pub mod comment;
pub mod story;
pub mod vote;

pub use comment::Entity as Comment;
pub use story::Entity as Story;
pub use vote::Entity as Vote;
