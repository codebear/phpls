use std::fs;
use std::fs::DirEntry;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

pub struct FileScanner {
    root_path: PathBuf,
}

impl FileScanner {
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let path = std::fs::canonicalize(path)?;
        Ok(FileScanner { root_path: path })
    }

    pub fn recurse_with_filter(
        &self,
        filter: &dyn Fn(&DirEntry) -> bool,
        callback: &mut dyn FnMut(&DirEntry),
    ) -> io::Result<()> {
        let local_path: PathBuf = self.root_path.clone();
        eprintln!("Starter scanning av {}", local_path.display());
        let mut paths = vec![Arc::new(local_path)];
        while let Some(path) = paths.pop() {
            //    eprintln!("Fant path on stack: {}", path.display());
            for entry in fs::read_dir(&*path)? {
                let entry = entry?;
                //                eprintln!("Fant entry: {:?}", entry);
                let e_path = Arc::new(entry.path());
                if e_path.is_dir() {
                    paths.push(e_path);
                } else if e_path.is_file() {
                    if filter(&entry) {
                        callback(&entry);
                    }
                }
            }
        }
        Ok(())
    }
}
