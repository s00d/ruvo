//! Local filesystem for Sova — jail root, async CRUD, walk.
//!
//! ```ignore
//! app.install(Fs::new("./data"));
//! let kids = req.fs().read_dir("notes").await?;
//! req.fs().write("notes/a.txt", b"hi").await?;
//! ```

mod error;
mod events;
mod fs;
mod path;

pub use error::FsError;
pub use events::{DirCreated, FileRemoved, FileWritten};
pub use fs::{Fs, FsEntry, FsExt, FsMeta, FsPlugin};
