#![recursion_limit = "256"]

use phpanalyzer::issue::VoidEmitter;
use phpanalyzer::symboldata::SymbolData;

use crate::codetree::codetree::CodeTree;
use crate::codetree::workspace::Workspace;
use crate::config::PHPLintConfig;
use crate::issues::OutputEmitter;
use crate::phpls::stdioserver::PHPStdIOLanguageServer;
use crate::phpls::tcpserver::PHPTCPLanguageServer;
use crate::phpparser::phpfile::PHPFile;
use std::collections::VecDeque;
use std::io::Error;
use std::io::ErrorKind;
// use crate::codetree::codetree::CodeTree;

use std::env;
use std::sync::Arc;

mod codetree;
mod config;
mod issues;
mod phpls;
mod phpparser;
mod storage;


// #[cfg(test)]
// mod tests;

struct PHPLintProgram {
    cmdname: String,
}

impl PHPLintProgram {
    fn start_server(&self, server_type: String) -> Result<(), String> {
        match server_type.as_str() {
            "tcp" => PHPTCPLanguageServer::new().start_tcp_server(),
            "stdio" => PHPStdIOLanguageServer::new().start_stdio_server(),
            _ => Err(format!("Unknown server type {}", server_type)),
        }
    }

    fn analyze_file(&self, filename: String) -> std::io::Result<()> {
        eprintln!("Her skal vi analysere {}", filename);
        let file = PHPFile::new(filename.into());
        eprintln!("File initialized");
        let mut emitter = OutputEmitter::new();
        if let Ok(symbols) = file.analyze(&mut emitter) {
            eprintln!("Done analyzing");
            Ok(())
        } else {
            eprintln!("Done analyzing");
            Err(Error::new(
                ErrorKind::Other,
                "Someone should extract an error here",
            ))
        }
    }

    fn parse2_file(&self, filename: String) {
        eprintln!("Her skal vi analysere {}", filename);
        let file = PHPFile::new(filename.into());
        eprintln!("File initialized");
        let emitter = OutputEmitter::new();
        file.parse2(&emitter);
        eprintln!("Done analyzing");
    }

    fn dump_ast_file(&self, filename: String) -> Result<(), Error> {
        eprintln!("Her skal vi dump {}", filename);
        let file = PHPFile::new(filename.into());
        file.dump_ast(&mut VoidEmitter::new())
    }

    fn traverse_folder(
        &self,
        folder: String,
        thread_count: usize,
        output_issues: bool,
    ) -> std::io::Result<Arc<SymbolData>> {
        eprintln!(
            "Her skal vi traversere {} med {} threads",
            &folder, thread_count
        );
        let code_tree = CodeTree::new(folder.into());
        let pre = std::time::Instant::now();
        let res = code_tree.traverse(thread_count, output_issues);

        let clock = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .ok()
            .unwrap_or_else(|| std::time::Duration::new(0, 0));
        eprintln!("Elapsed: {}ms ({:?})", pre.elapsed().as_millis(), clock);
        res
    }

    fn usage(&self) {
        eprintln!("   {} [--server type| --analyze filename | --describe filename lineno charpos | [--output-issues] --traverse folder]", self.cmdname);
    }

    fn main(&self, mut args: VecDeque<String>) -> i32 {
        let config = match PHPLintConfig::default_from_cwd() {
            Ok(config) => config,
            Err(_) => {
                eprintln!("Error: Couldn't establish configuration!?!");
                self.usage();
                return 1;
            }
        };

        // Container to keep all the jobs we will perform once the cmdline parsing is complete
        let mut tasks: Vec<Box<dyn FnOnce() -> std::io::Result<()>>> = vec![];
        let mut threads: Option<usize> = None;
        let mut output_issues: bool = false;
        let mut dump_cache: bool = false;

        while args.len() > 0 {
            let arg = args
                .pop_front()
                .expect("when args.len() > 0 this should not fail!");
            match arg.as_str() {
                "--server" => {
                    if let Some(server_type) = args.pop_front() {
                        tasks.push(Box::new(|| {
                            if let Err(er) = self.start_server(server_type) {
                                self.usage();
                                Err(Error::new(ErrorKind::Other, format!("Error: {}", er)))
                            } else {
                                Ok(())
                            }
                        }))
                    }
                }
                "--analyze" => {
                    if let Some(filename) = args.pop_front() {
                        tasks.push(Box::new(|| self.analyze_file(filename)));
                    } else {
                        eprintln!("Error: Missing filename to `--analyze`");
                        self.usage();
                        return -1;
                    }
                }
                "--parse" => {
                    if let Some(filename) = args.pop_front() {
                        self.parse2_file(filename);
                    }
                }
                "--dump" => {
                    if let Some(filename) = args.pop_front() {
                        tasks.push(Box::new(|| self.dump_ast_file(filename)))
                    } else {
                        eprintln!("Error: Missing filename to `--analyze`");
                        self.usage();
                        return -1;
                    }
                }
                "--threads" => {
                    // Her burde vi match i stedet
                    if let Some(Ok(thread_count)) =
                        args.pop_front().map(|s| str::parse::<usize>(s.as_str()))
                    {
                        threads = Some(thread_count);
                    } else {
                        eprintln!("Error: Missing valid thread count argument to `--threads`");
                        self.usage();
                        return -1;
                    }
                }
                "--output-issues" => {
                    output_issues = true;
                }
                "--dump-cache" => {
                    dump_cache = true;
                }
                "--traverse" => {
                    if let Some(root_folder) = args.pop_front() {
                        let thread_count = threads.clone().unwrap_or(1);
                        tasks.push(Box::new(move || {
                            let res =
                                self.traverse_folder(root_folder, thread_count, output_issues);
                            let clock = std::time::SystemTime::now()
                                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                .ok()
                                .unwrap_or_else(|| std::time::Duration::new(0, 0));
                            eprintln!("CLOCK EHERE {:?}", clock);
                            let symbols = res?;

                            if dump_cache {
                                cerum::cache::dump_cache(symbols);
                            }

                            Ok(())
                        }));
                    } else {
                        eprintln!("Error: Missing folder to `--traverse`");
                        self.usage();
                        return -1;
                    }
                }
                "--help" => {
                    self.usage();
                    return 0;
                }
                "--describe" => {
                    let file = args.pop_front();
                    let line = args.pop_front().map(|s| str::parse::<u32>(s.as_str()));
                    let cchar = args.pop_front().map(|s| str::parse::<u32>(s.as_str()));
                    match (file, line, cchar) {
                        (Some(filename), Some(Ok(lineno)), Some(Ok(charpos))) => {
                            let config = config.clone();
                            tasks.push(Box::new(move || {
                                Workspace::new(config).describe_position(filename, lineno, charpos)
                            }))
                        }
                        _ => {
                            eprintln!(
                                "error: bad arguments to --describe <file> <lineno> <charpos>"
                            );
                            self.usage();
                            return 1;
                        }
                    }
                }
                _ => {
                    eprintln!("Error: Unknown argument {}", arg);
                    self.usage();
                    return 1;
                }
            }
        }
        if tasks.is_empty() {
            self.usage();
            return 1;
        }

        // execute all tasks
        let pre = std::time::Instant::now();
        for task in tasks {
            eprintln!("Starting task... ({}ms)", pre.elapsed().as_millis());
            if let Err(e) = task() {
                eprintln!("ERROR: {}", e);
                self.usage();
                phpanalyzer::dump_missing_stats();
                return -1;
            }
            let clock = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .ok()
                .unwrap_or_else(|| std::time::Duration::new(0, 0));
            eprintln!(
                "Completed task ({}ms), {:?}",
                pre.elapsed().as_millis(),
                clock
            );
        }
        eprintln!("Completed all tasks ({}ms)", pre.elapsed().as_millis());
        phpanalyzer::dump_missing_stats();
        return 0;
    }
}

fn main() {
    let mut args: VecDeque<String> = env::args().collect();
    let cmdname = args
        .pop_front()
        .expect("It should not be possible to start a program with empty args-vector");
    let program = PHPLintProgram { cmdname: cmdname };
    std::process::exit(program.main(args));
}
