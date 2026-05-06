mod django;
mod fastapi;
mod flask;
mod node;

use crate::project::{FileEntry, Project, Stack};

pub fn files_for(project: &Project) -> Vec<FileEntry> {
    match project.stack {
        Stack::Node => node::files(project),
        Stack::Django => django::files(project),
        Stack::Flask => flask::files(project),
        Stack::Fastapi => fastapi::files(project),
    }
}
