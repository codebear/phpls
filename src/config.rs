use std::env;
use std::path::PathBuf;

#[derive(Clone)]
pub struct PHPLintConfig {
    pub root_folder: PathBuf,
    pub threads: usize,
}

impl PHPLintConfig {
    pub fn default_from_cwd() -> std::io::Result<Self> {
        let path = env::current_dir()?;

        Ok(PHPLintConfig {
            root_folder: path,
            threads: 8,
        })
    }
}
