use std::path::PathBuf;
use std::sync::RwLock;

use phpanalyzer::issue::Issue;
use phpanalyzer::issue::IssueEmitter;

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
