use crate::codetree::codetree::CodeTree;
use crate::PHPLintConfig;
use std::convert::TryInto;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

pub struct Workspace {
    pub config: PHPLintConfig,
    codetree: CodeTree,
}

impl Workspace {
    pub fn new(config: PHPLintConfig) -> Self {
        Workspace {
            codetree: CodeTree::new(config.root_folder.clone()),
            config: config,
        }
    }

    pub fn describe_position(
        &self,
        filename: String,
        lineno: u32,
        charpos: u32,
    ) -> std::io::Result<()> {
        if lineno < 1 {
            return Err(Error::new(ErrorKind::Other, "Lineno can't be less than 1"));
        }
        if charpos < 1 {
            return Err(Error::new(ErrorKind::Other, "Charpos can't be less than 1"));
        }
        let lineno: usize = (lineno - 1).try_into().unwrap();
        let charpos: usize = (charpos - 1).try_into().unwrap();
        let file = PathBuf::from(filename.clone());
        if let Some(php) = self.codetree.analyze_file(&file) {
            let symbol_data = self.codetree.get_symbol_data();
            match php.describe_pos(lineno, charpos, symbol_data) {
                Ok(Some(desc)) => eprintln!("DESCRIPTION: {}", desc),
                Ok(None) => eprintln!("Nothing to describe at position"),
                Err(e) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Error while describing: {}", e),
                    ))
                }
            }
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("Finner ikke filen {}", filename),
            ))
        }
    }
}
