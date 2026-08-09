//! Domain events for filesystem mutations.

use sova_core::Event;

#[derive(Debug, Clone)]
pub struct FileWritten {
    pub path: String,
}

impl Event for FileWritten {
    fn name(&self) -> &'static str {
        "fs.file_written"
    }
}

#[derive(Debug, Clone)]
pub struct FileRemoved {
    pub path: String,
}

impl Event for FileRemoved {
    fn name(&self) -> &'static str {
        "fs.file_removed"
    }
}

#[derive(Debug, Clone)]
pub struct DirCreated {
    pub path: String,
}

impl Event for DirCreated {
    fn name(&self) -> &'static str {
        "fs.dir_created"
    }
}
