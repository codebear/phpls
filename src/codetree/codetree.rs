use crate::codetree::file_scanner::FileScanner;
use crate::issues::OutputEmitter;
use crate::phpparser::phpfile::PHPFile;
use php_tree_sitter::analysis::state::AnalysisState;
use php_tree_sitter::issue::{Issue, IssueEmitter};
use php_tree_sitter::symboldata::SymbolData;
use std::ffi::OsString;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use url::Url;

pub trait GenericProgress {
    fn set_max(&self, max: usize);
    fn progress(&self, str: &str);
}

#[derive()]
pub struct CallbackProgress {
    max: AtomicUsize,
    prev_ident: Arc<RwLock<Option<String>>>,
    count: Arc<RwLock<usize>>,
    prev_output: Arc<RwLock<usize>>,
    callback: Box<dyn Fn(usize, String) -> () + Send + Sync>,
}

impl CallbackProgress {
    pub fn new(callback: Box<dyn Fn(usize, String) -> () + Send + Sync>) -> Self {
        Self::new_with_max(callback, 0)
    }

    pub fn new_with_max(
        callback: Box<dyn Fn(usize, String) -> () + Send + Sync>,
        max: usize,
    ) -> Self {
        Self {
            max: AtomicUsize::new(max),
            prev_ident: Arc::new(RwLock::new(None)),
            count: Arc::new(RwLock::new(0)),
            prev_output: Arc::new(RwLock::new(0)),
            callback,
        }
    }
}

impl GenericProgress for CallbackProgress {
    fn set_max(&self, max: usize) {
        self.max.store(max, Ordering::Relaxed);
    }

    fn progress(&self, ident: &str) {
        let ident: String = ident.into();
        {
            let mut cnt = self.count.write().unwrap();

            let mut new = false;
            let mut reset = false;
            {
                let read_ident = self.prev_ident.read().unwrap();
                if let Some(x) = &*read_ident {
                    if *x != ident {
                        new = true;
                        reset = true;
                    }
                } else {
                    new = true;
                }
            }
            if new {
                let mut wr = self.prev_ident.write().unwrap();
                (*wr) = Some(ident.clone());
            }
            if reset {
                *cnt = 0;
            }

            *cnt += 1;
        }
        let read = self.count.read().unwrap();
        let self_max = self.max.load(Ordering::Relaxed);
        let max = if *read > self_max { *read } else { self_max };
        let percent = if max > 0 { *read * 100 / max } else { 0 };
        if percent != *self.prev_output.read().unwrap() {
            (self.callback)(percent, ident);
            *self.prev_output.write().unwrap() = percent;
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaptureEmitter {
    issues: Arc<RwLock<Vec<Issue>>>,
}

impl CaptureEmitter {
    pub fn new() -> Self {
        Self {
            issues: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn get_issues(&self) -> Vec<Issue> {
        let handle = self.issues.read().unwrap();
        handle.clone()
    }
}

impl IssueEmitter for CaptureEmitter {
    fn emit(&self, issue: Issue) {
        let mut write = self.issues.write().unwrap();
        write.push(issue);
    }
}
pub struct CodeTree {
    pub root_folder: PathBuf,

    pub files: Arc<RwLock<Vec<Arc<PHPFile>>>>,
    pub symbol_data: Arc<RwLock<Option<Arc<SymbolData>>>>,
    pub issues: Arc<RwLock<Vec<Issue>>>,
}

pub struct Worker<T> {
    pub handle: JoinHandle<()>,
    pub sender: Sender<T>,
}

impl CodeTree {
    pub fn new(root_folder: PathBuf) -> CodeTree {
        CodeTree {
            root_folder: root_folder,
            files: Arc::new(RwLock::new(vec![])),
            symbol_data: Arc::new(RwLock::new(None)),
            issues: Arc::new(RwLock::new(vec![])),
        }
    }

    pub fn new_for_url(url: Url, _name: String) -> Option<CodeTree> {
        if url.scheme() != "file" {
            return None;
        }

        Some(CodeTree {
            root_folder: PathBuf::from(url.path()),
            files: Arc::new(RwLock::new(vec![])),
            symbol_data: Arc::new(RwLock::new(None)),
            issues: Arc::new(RwLock::new(vec![])),
        })
    }

    pub fn get_issues_for_uri(&self, uri: &Url) -> Vec<Issue> {
        // FIXME this is probably not an ideal search-algorithm
        let issues = self.issues.read().unwrap();
        if uri.scheme() != "file" {
            return vec![];
        }
        let s = uri.path();
        let uri_as_osstring: OsString = s.into();
        eprintln!(
            "Looking for issues matching {:?} out of {} total",
            uri_as_osstring,
            issues.len()
        );
        issues
            .iter()
            .filter(|x| x.issue_file() == uri_as_osstring)
            .cloned()
            .collect()
    }

    pub fn traverse(&self, thread_count: usize) -> std::io::Result<()> {
        let output_issues = true;
        let capture_emitter = Arc::new(CaptureEmitter::new());
        let emitter: Arc<dyn IssueEmitter + Send + Sync> = if output_issues {
            Arc::new(OutputEmitter::new())
        } else {
            capture_emitter.clone()
        };
        let symbol_data = Arc::new(SymbolData::new());
        let status = Arc::new(CallbackProgress::new(Box::new(|percent, ident| {
            eprint!("\x1b[1G{:-3}% {}    \x1b[1G", percent, ident);
        })));

        let res = self.internal_traverse(thread_count, symbol_data, emitter.clone(), status);
        if !output_issues {
            use itertools::Itertools;
            eprintln!("\nSummary of issues:\n");
            for (_key, group) in capture_emitter
                .get_issues()
                .iter()
                .map(|i| (std::mem::discriminant(i), i))
                .into_group_map()
            {
                let ident = group[0].get_name();
                eprintln!(" *  {}: {}", ident, group.len())
                // void
            }
            eprintln!("\n");
        }
        res
    }

    pub fn run_analysis(
        &self,
        thread_count: usize,
        status: Arc<dyn GenericProgress + Send + Sync>,
    ) -> std::io::Result<()> {
        let emitter = Arc::new(CaptureEmitter::new());
        let symbol_data = Arc::new(SymbolData::new());

        self.internal_traverse(thread_count, symbol_data.clone(), emitter.clone(), status)?;

        let mut sd_handle = self.symbol_data.write().unwrap();
        (*sd_handle) = Some(symbol_data);
        let mut issues_handle = self.issues.write().unwrap();
        (*issues_handle) = emitter.get_issues();
        Ok(())
    }

    pub fn internal_traverse(
        &self,
        thread_count: usize,
        symbol_data: Arc<SymbolData>,
        emitter: Arc<dyn IssueEmitter + Send + Sync>,
        status: Arc<dyn GenericProgress + Send + Sync>,
    ) -> std::io::Result<()> {
        if thread_count == 0 {
            eprintln!("Zero thread count");
            return Err(Error::new(ErrorKind::Other, "Thread count can't be 0"));
        }
        if thread_count > 64 {
            eprintln!("Thread count is max 64");
            return Err(Error::new(ErrorKind::Other, "Thread count is max 64"));
        }
        let mut new_files = vec![];
        let base_lib = "/Users/bear/src/cerum/src/lib";
        let base_libs = "/Users/bear/src/cerum/src/libs";
        let base_sec = "/Users/bear/src/cerum/src/sec";
        self.traverse_disk_in_thread(&mut |file| {
            if !(file.starts_with(base_lib)
                || file.starts_with(base_libs)
                || file.starts_with(base_sec))
            {
                return;
            }
            if file.is_file() {
                new_files.push(Arc::new(PHPFile::new(file)));
            }
        })?;

        *(self.files.write().unwrap()) = new_files;
        let files = self.files.clone();
        status.set_max(files.read().unwrap().len());
        let mut state = AnalysisState::new_with_symbols(symbol_data.clone());
        php_tree_sitter::native::register(&mut state);

        if thread_count > 1 {
            let thread_emitter = emitter.clone();
            let thread_symbol = symbol_data.clone();
            let status = status.clone();
            self.traverse_list_with_threads(
                thread_count,
                Arc::new(move |file| {
                    file.analyze_round_one(&*thread_emitter, thread_symbol.clone());
                    status.progress("pass 1/2");
                }),
            )?;
        } else {
            let thread_emitter = emitter.clone();
            let status = status.clone();
            let thread_symbol = symbol_data.clone();

            self.traverse_list_in_thread(Box::new(move |file| {
                file.analyze_round_one(&*thread_emitter, thread_symbol.clone());
                status.progress("pass 1/2");
            }))?;
        }

        if thread_count > 1 {
            let status = status.clone();
            let thread_emitter = emitter.clone();
            let thread_symbol = symbol_data.clone();

            self.traverse_list_with_threads(
                thread_count,
                Arc::new(move |file| {
                    file.analyze_round_two(&*thread_emitter, thread_symbol.clone());
                    status.progress("pass 2/2");
                }),
            )?;
        } else {
            let status = status.clone();
            let thread_emitter = emitter.clone();
            let thread_symbol = symbol_data.clone();
            self.traverse_list_in_thread(Box::new(move |file| {
                file.analyze_round_two(&*thread_emitter, thread_symbol.clone());
                status.progress("pass 2/2");
            }))?;
        }

        Ok(())
    }

    pub fn traverse_disk_in_thread(
        &self,
        callback: &mut dyn FnMut(PathBuf),
    ) -> std::io::Result<()> {
        let scanner = FileScanner::new(self.root_folder.clone())?;
        scanner.recurse_with_filter(
            &|entry| {
                entry
                    .path()
                    .extension()
                    .map_or(false, |f| f.eq("php") || f.eq("php3"))
            },
            &mut |php_file| {
                callback(php_file.path());
            },
        )
    }

    pub fn traverse_list_in_thread(
        &self,
        callback: Box<dyn Fn(Arc<PHPFile>) + Send + Sync>,
    ) -> std::io::Result<()> {
        let reader = match self.files.read() {
            Ok(r) => r,
            Err(_e) => todo!("CRAP"),
        };

        for file in &*reader {
            callback(file.clone());
        }
        Ok(())
    }

    pub fn traverse_disk_with_threads(
        &self,
        thread_count: usize,
        callback: Arc<dyn Fn(PathBuf) + Send + Sync>,
    ) -> std::io::Result<()> {
        let mut workers: Vec<Worker<PathBuf>> = vec![];

        for _ in 0..thread_count {
            let (tx, rx) = channel::<PathBuf>();
            let thread_callback = callback.clone();
            let handle = thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(file) => {
                            // eprintln!("Should analyze file {:?}", file);
                            thread_callback(file);
                        }
                        Err(_) => {
                            // eprintln!("ERROR: {:?}", e);
                            break;
                        }
                    }
                }
            });
            workers.push(Worker { handle, sender: tx });
        }
        let scanner = FileScanner::new(self.root_folder.clone())?;
        let mut cnt: usize = 0;
        scanner.recurse_with_filter(
            &|entry| {
                entry
                    .path()
                    .extension()
                    .map_or(false, |f| f.eq("php") || f.eq("php3"))
            },
            &mut |php_file| {
                workers[cnt % workers.len()]
                    .sender
                    .send(php_file.path())
                    .unwrap();
                cnt += 1;
                //                self.traverse_file(php_file)
            },
        )?;
        for worker in workers {
            drop(worker.sender);
            match worker.handle.join() {
                Ok(_) => (),
                Err(err) => return Err(Error::new(ErrorKind::Other, format!("Crap {:?}", err))),
            };
        }
        eprintln!("Completed.");
        Ok(())
    }

    pub fn traverse_list_with_threads(
        &self,
        thread_count: usize,
        callback: Arc<dyn Fn(Arc<PHPFile>) + Send + Sync>,
    ) -> std::io::Result<()> {
        let mut workers: Vec<Worker<Arc<PHPFile>>> = vec![];

        for _ in 0..thread_count {
            let iter_callback = callback.clone();
            let (tx, rx) = channel::<Arc<PHPFile>>();
            let handle = thread::spawn(move || {
                loop {
                    match rx.recv() {
                        Ok(file) => {
                            // eprintln!("Should analyze file {:?}", file);
                            iter_callback(file);
                        }
                        Err(_) => {
                            // eprintln!("ERROR: {:?}", e);
                            break;
                        }
                    }
                }
            });
            workers.push(Worker { handle, sender: tx });
        }

        let reader = match self.files.read() {
            Ok(r) => r,
            Err(_e) => todo!("CRAP"),
        };
        let mut cnt: usize = 0;

        for file in &*reader {
            workers[cnt % workers.len()]
                .sender
                .send(file.clone())
                .unwrap();
            cnt += 1;
        }

        for worker in workers {
            drop(worker.sender);
            match worker.handle.join() {
                Ok(_) => (),
                Err(err) => return Err(Error::new(ErrorKind::Other, format!("Crap {:?}", err))),
            };
        }
        eprintln!("Completed.");
        Ok(())
    }

    pub fn contains_file(&self, _file: &Url) -> bool {
        true
        /*        if file.scheme() != "file" {
            eprintln!("{} is not in the file-schema", file);
            return false;
        }
        let path = PathBuf::from(file.path());
        if !path.starts_with(&self.root_folder) {
            eprintln!("{} is not contained in {:?}", file, self.root_folder);
            return false;
        }
        path.is_file()*/
    }

    pub fn traverse_file(&self, file: PathBuf) -> Result<(), Error> {
        // eprintln!("Found {:?}", &file);

        let php_file = if file.is_file() {
            PHPFile::new(file)
        } else {
            return Err(Error::new(ErrorKind::NotFound, format!("File {:?}", file)));
        };

        php_file.analyze(&mut OutputEmitter::new())
    }

    pub fn analyze_file(&self, file: &PathBuf) -> Option<PHPFile> {
        if file.is_file() {
            Some(PHPFile::new(file.clone()))
        } else {
            None
        }
    }

    pub fn analyze_file_uri(&self, file: &Url) -> Option<PHPFile> {
        if file.scheme() != "file" {
            return None;
        }
        self.analyze_file(&PathBuf::from(file.path()))
    }

    pub fn with_analyzed_file<CB>(file: &PathBuf, cb: CB)
    where
        CB: Fn(PHPFile),
    {
        let php_file = PHPFile::new(file.clone());
        cb(php_file);
    }

    pub(crate) fn get_symbol_data(&self) -> Option<Arc<SymbolData>> {
        let symbol_data = self.symbol_data.read().unwrap();

        symbol_data.clone()
    }
}
