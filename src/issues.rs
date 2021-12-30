use std::path::PathBuf;
use std::sync::RwLock;

use php_tree_sitter::issue::Issue;
use php_tree_sitter::issue::IssueEmitter;

pub struct OutputEmitter {
    pub file_name: RwLock<Option<PathBuf>>,
}

impl OutputEmitter {
    pub fn new() -> Self {
        OutputEmitter {
            file_name: RwLock::new(None),
        }
    }
}

impl IssueEmitter for OutputEmitter {
    fn emit(&self, issue: Issue) {
        eprintln!(
            "Issue: {:?} {}",
            issue.severity(),
            issue.as_string_with_pos()
        );
    }
}
