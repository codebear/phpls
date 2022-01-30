use phpanalyzer::analysis::analyzer::Analyzer;
use phpanalyzer::analysis::state::{AnalysisState, LookingForNode};
use phpanalyzer::autonodes::any::AnyNodeRef;
use phpanalyzer::issue::{IssueEmitter, VoidEmitter};
use phpanalyzer::symboldata::SymbolData;
use phpanalyzer::symbols::Symbol;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::RwLock;
/*use crate::php::nodes::method_node::MethodNode;
use crate::storage::symbols::SymbolMethod;
use crate::php::type_analysis::AnalyzeState;
use crate::storage::symbols::Symbol;
use crate::storage::symbols::SymbolClass;
use crate::storage::symbols::Position;
*/
use std::sync::Arc;
/*use crate::php::nodes::class_node::ClassNode;
use crate::phpparser::asttree::PHPAstTree;
use crate::storage::symbols::SymbolStorage;
use crate::php::unserializebuffer::UnserializeBuffer;
use crate::php::unserializer::php_unserialize;
use std::process::Command;
use crate::php::lookup::ClassLookup;
use crate::php::astnodeimpl::ClassProperties;
use crate::php::lookup::MethodLookup;
use crate::php::astnodeimpl::FunctionProperties;
use crate::php::type_analysis::NodeType;
*/

use phpanalyzer::description::NodeDescription;
use phpanalyzer::Point;

#[derive(Clone)]
pub struct PHPFile {
    pub fq_file_name: PathBuf,
    analyzed: Arc<RwLock<Option<Arc<Analyzer>>>>,
}

impl PHPFile {
    pub fn new(fq_file_name: PathBuf) -> PHPFile {
        return PHPFile {
            fq_file_name: fq_file_name,
            analyzed: Arc::new(RwLock::new(None)),
        };
    }

    pub fn analyze_with_callback_at_position<T_Result>(
        &self,
        line: usize,
        character: usize,
        symbol_data: Option<Arc<SymbolData>>,
        callback: Box<
            dyn FnOnce(AnyNodeRef, &mut AnalysisState, &Vec<AnyNodeRef>) -> T_Result + Send + Sync,
        >,
    ) -> Result<Option<T_Result>, &'static str>
    where
        T_Result: 'static + Send + Sync + Clone,
    {
        let a = if let Some(a) = self.get_analyzer() {
            a
        } else {
            return Err("Fikk ikke analysert fila");
        };

        let void_emitter = VoidEmitter::new();
        let symbol_data = symbol_data.unwrap_or_else(|| Arc::new(SymbolData::new()));

        // Pass 1
        let mut state = AnalysisState::new_with_symbols(symbol_data.clone());
        phpanalyzer::native::register(&mut state);

        a.first_pass(&mut state, &void_emitter);

        // Pass 2-1
        let mut state = AnalysisState::new_with_symbols(symbol_data.clone());
        phpanalyzer::native::register(&mut state);
        a.second_pass(&mut state, &void_emitter);

        // Pass 2-2
        let mut state = AnalysisState::new_with_symbols(symbol_data);
        phpanalyzer::native::register(&mut state);
        let result_container = Arc::new(RwLock::new(None));

        let result_container_copy = result_container.clone();
        let looking_for = LookingForNode {
            pos: Point {
                row: line,
                column: character,
            },
            callback: Arc::new(RwLock::new(Some(Box::new(move |node, state, path| {
                let mut writable = result_container_copy.write().unwrap();
                let result = callback(node, state, path);
                *writable = Some(result);
            })))),
        };
        state.looking_for_node = Some(looking_for);
        a.third_pass(&mut state, &void_emitter);

        let mut writeable_result = result_container.write().unwrap();

        if let Some(result) = writeable_result.take() {
            return Ok(Some(result));
        } else {
            eprintln!("No hits for looking for {}:{}", line, character);
        }

        Ok(None)
    }

    pub fn describe_pos(
        &self,
        line: usize,
        character: usize,
        symbol_data: Option<Arc<SymbolData>>,
    ) -> Result<Option<String>, &'static str> {
        let a = if let Some(a) = self.get_analyzer() {
            a
        } else {
            return Err("Fikk ikke analysert fila");
        };

        let void_emitter = VoidEmitter::new();
        let symbol_data = symbol_data.unwrap_or_else(|| Arc::new(SymbolData::new()));

        // Pass 1
        let mut state = AnalysisState::new_with_symbols(symbol_data.clone());
        phpanalyzer::native::register(&mut state);

        a.first_pass(&mut state, &void_emitter);

        // Pass 2-1
        let mut state = AnalysisState::new_with_symbols(symbol_data.clone());
        phpanalyzer::native::register(&mut state);
        a.second_pass(&mut state, &void_emitter);

        // Pass 2-2
        let mut state = AnalysisState::new_with_symbols(symbol_data);
        phpanalyzer::native::register(&mut state);
        let result_container = Arc::new(RwLock::new(None));

        let result_container_copy = result_container.clone();
        let looking_for = LookingForNode {
            pos: Point {
                row: line,
                column: character,
            },
            callback: Arc::new(RwLock::new(Some(Box::new(move |node, state, path| {
                let mut writable = result_container_copy.write().unwrap();
                *writable = node.description(Some(&path[..]), state);
            })))),
        };
        state.looking_for_node = Some(looking_for);
        a.third_pass(&mut state, &void_emitter);

        if let Some(tekst) = &*result_container.read().unwrap() {
            return Ok(Some(format!("FANT VIA ANALYZE: {}", tekst)));
        } else {
            eprintln!("No hits for looking for {}:{}", line, character);
        }

        Ok(None)
    }

    pub fn create_analyzer(&self) -> Analyzer {
        let fname = self.fq_file_name.clone();
        Analyzer::new(
            Box::new(move || std::fs::read(&fname)),
            self.fq_file_name.as_os_str().to_os_string(),
        )
    }

    pub fn parse2(&self, _emitter: &dyn IssueEmitter) -> Option<Arc<Analyzer>> {
        let mut a = self.create_analyzer();
        a.parse();
        None
    }

    fn get_analyzer(&self) -> Option<Arc<Analyzer>> {
        // Read lock scope
        {
            let state = self.analyzed.read().unwrap();

            if state.is_some() {
                return state.clone();
            }
        }

        // Write lock scope
        {
            let mut locked = self.analyzed.write().unwrap();
            if locked.is_some() {
                // someone beat us to it
                return locked.clone();
            }
            let mut a = self.create_analyzer();
            match a.parse() {
                Ok(_) => {
                    let val = Some(Arc::new(a));
                    *locked = val.clone();
                    val
                }
                Err(e) => panic!("ERR: {}", e),
            }
        }
    }

    pub fn analyze_first_pass(&self, emitter: &dyn IssueEmitter, symbol_data: Arc<SymbolData>) {
        if let Some(analyzer) = self.get_analyzer() {
            let mut state = AnalysisState::new_with_symbols(symbol_data);
            state.filename = Some(self.fq_file_name.clone());
            state.pass = 1;
            analyzer.first_pass(&mut state, emitter);
        }
    }

    pub fn analyze_second_pass(&self, emitter: &dyn IssueEmitter, symbol_data: Arc<SymbolData>) {
        if let Some(analyzer) = self.get_analyzer() {
            let mut state = AnalysisState::new_with_symbols(symbol_data);
            state.filename = Some(self.fq_file_name.clone());
            state.pass = 1;
            analyzer.second_pass(&mut state, emitter);
        }
    }


    pub fn analyze_third_pass(&self, emitter: &dyn IssueEmitter, symbol_data: Arc<SymbolData>) {
        if let Some(analyzer) = self.get_analyzer() {
            let mut state = AnalysisState::new_with_symbols(symbol_data);
            state.pass = 2;
            state.filename = Some(self.fq_file_name.clone());
            analyzer.third_pass(&mut state, emitter);
        }
    }

    ///
    /// Analyze this file with symbol-data
    ///
    pub fn analyze_with_symbol_data(
        &self,
        emitter: &dyn IssueEmitter,
        symbol_data: Arc<SymbolData>,
    ) -> std::io::Result<()> {
        let mut state = AnalysisState::new_with_symbols(symbol_data.clone());
        phpanalyzer::native::register(&mut state);
        self.analyze_first_pass(emitter, symbol_data.clone());
        self.analyze_third_pass(emitter, symbol_data);
        Ok(())
    }

    ///
    /// Analyze this file standalone
    ///
    pub fn analyze(&self, emitter: &dyn IssueEmitter) -> std::io::Result<()> {
        let symbol_data = Arc::new(SymbolData::new());
        self.analyze_with_symbol_data(emitter, symbol_data)
    }

    /*pub fn analyze(&self, emitter: &mut impl IssueEmitter) -> Option<Arc<Analyzer>> {
        emitter.set_file_name(&self.fq_file_name);
        // Read lock scope
        {
            let state = self.analyzed.read().unwrap();

            if state.is_some() {
                return state.clone();
            }
        }
        // Write lock scope
        {
            let mut locked = self.analyzed.write().unwrap();
            if locked.is_some() {
                // someone beat us to it
                return locked.clone();
            }
            let mut a = Analyzer::new(self.clone());
            a.parse();
            match a.analyze(emitter) {
                Ok(_) => {
                    let val = Some(Arc::new(a));
                    *locked = val.clone();
                    val
                },
                Err(e) => {
    //                eprintln!("Error parsing: {}", e);
                    None
                }
            }
        }
    }*/

    /*pub fn analyze(&mut self, emitter: &dyn IssueEmitter) -> Result<(), &'static str> {
        self.parse()?;
        let mut state = AnalysisState::new();
        state.filename = Some(self.file.fq_file_name.as_os_str().to_os_string());
        self.round_one(&mut state, emitter);
        self.round_two(&mut state, emitter);
        Ok(())
    }*/

    pub fn dump_ast(&self, _emitter: &mut dyn IssueEmitter) -> std::io::Result<()> {
        if let Some(a) = self.get_analyzer() {
            a.dump();
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Other, format!("Find an error")))
        }
    }

    pub(crate) fn get_symbols_at(
        &self,
        line: usize,
        charpos: usize,
        symbol_data: Option<Arc<SymbolData>>,
    ) -> Option<Vec<Symbol>> {
        let result = self.analyze_with_callback_at_position(
            line,
            charpos,
            symbol_data,
            Box::new(Self::get_symbols_at_callback),
        );
        // eprintln!("ETTERPA HAR VI: {:?}", result);
        result.ok()??
    }

    fn get_symbols_at_callback(
        found_node: AnyNodeRef,
        state: &mut AnalysisState,
        path: &Vec<AnyNodeRef>,
    ) -> Option<Vec<Symbol>> {
        // void
        let mut node = &found_node;

        //         let path = path.clone();
        let mut iter = path.iter().rev();
        loop {
            eprintln!("Ser etter symbol i en {:?}", node.kind());
            return match node {
                AnyNodeRef::MemberCallExpression(e) => e.get_symbols(state),
                AnyNodeRef::MemberAccessExpression(e) => e.get_symbols(state),
                _ => {
                    if let Some(n) = iter.next() {
                        node = n;
                        continue;
                    } else {
                        None
                    }
                }
            };
        }
    }

    /* pub fn parse_file(&self) -> Result<PHPAstTree, &'static str> {
            use phpanalyzer::parser::PHPParser;
            use std::fs;
            let mut parser = PHPParser::new();

            let contents = fs::read(&self.fq_file_name)
                .expect("Something went wrong reading the file");

            if let Some(tree) = parser.parse_wrapped(contents, None) {
                PHPAstTree::from_tree_sitter(tree)
            } else {
                Err("TODO Trouble with something")
            }
        }

        pub fn old_parse_file(&self) -> Result<PHPAstTree, &'static str> {
            // Old parse
            self.unserialize_raw_ast_tree(self.get_raw_serialized_ast()?)
        }

        fn get_raw_serialized_ast(&self) -> Result<std::vec::Vec<u8>, &'static str> {
            let php = "/usr/local/bin/php";
            eprintln!("Genererer AST");


            let result = Command::new(php)
                .arg("/Users/bear/src/phplint/src/parse.php")
                .arg(self.fq_file_name.clone())
                .output()
                .expect("failed to execute process");
            eprintln!("Konverterer til intern AST {}", result.status);
            if !result.status.success() {
                eprintln!("error parsing php-file.");
                eprintln!(
                    "Error: {:?}",
                    String::from_utf8_lossy(result.stderr.as_slice())
                );
                eprintln!(
                    "Output: {:?}",
                    String::from_utf8_lossy(result.stdout.as_slice())
                );
                return Err("Crap");
            }
            return Ok(result.stdout);
        }

        fn unserialize_raw_ast_tree(&self, raw_ast_buffer: std::vec::Vec<u8>) -> Result<PHPAstTree, &'static str> {
            if let Ok(tree) = php_unserialize(&mut UnserializeBuffer {
                buf: raw_ast_buffer,
                ptr: 0,
            }) {
                if let Some(tree) = PHPAstTree::from_struct(tree) {
                    return Ok(tree);
                } else {
                    Err("Fikk ikke parset AST-tree")
                }
            } else {
                eprintln!("php_unserialize() failed");
                return Err("PHP Unserialize failed");
            }
        }

        pub fn dump_debug_out(&self, tree: PHPAstTree) -> Result<String, &'static str> {
            let symbols = match SymbolStorage::new() {
                Ok(symbols) => symbols,
                Err(e) => {
                    eprintln!("NEI Får ikke symbolstorage {:?}", e);
                    return Err("fikk ikke symbolstorage");
                }
            };

            eprintln!("Dumping symbols");
            //eprintln!("Tree: {:?}", tree);
            if let Some(clses) = tree.get_all_classes() {
                for cls_e in clses {
                    self.dump_debug_class(&symbols, cls_e)?;
                }
            }
            eprintln!("Done dumping symbols.");
            return Ok("Done.".to_string());

        }

        pub fn dump_debug_class(&self, symbols: &SymbolStorage, cls: Arc<ClassNode>) -> Result<&'static str, &'static str> {
            eprint!("Class: {:?}", cls.get_class_name().unwrap());
            let lineno: u32 = cls.lineno;
            let pos: Position = Position::new(self.fq_file_name.clone(), lineno);

            let cname: String = cls.get_class_name()?;
            let ns: String = String::from("");
            let class = SymbolClass::new(cname, ns);
            match symbols.add_symbol(Symbol::Class(class.clone()), pos) {
                Ok(noe) => eprintln!(" [ added ] {}", noe),
                Err(e) => {
                    if e.to_string() == "UNIQUE constraint failed: symbol_class.name, symbol_class.namespace" {
                        eprintln!(" [  ok   ] ");
                    } else {
                        eprintln!(" [ error ] {}", e)
                    }
                }
            }

            match cls.get_all_methods() {
                Some(funcs) => {
                    for f in funcs {
                       self.dump_debug_method(symbols, &class, f)?;
                    }
                }
                _ => {
                    eprintln!("  ** nada");
                }
            }
            Ok("Ok")
        }

        pub fn dump_debug_method(&self, symbols: &SymbolStorage, class: &SymbolClass, f: Arc<MethodNode>) -> Result<&'static str, &'static str> {
            eprint!("  Function: {:?}: ",
                f.get_function_name().unwrap()
            );
            if let Ok(Some(types)) = f.get_node_type(&mut AnalyzeState::new(), &symbols) {
                eprint!("\n     Returns: {:?}", types);
        /*                                            for t in types.iter() {
                    eprint!(" {}", t);
                }*/
                eprintln!("");
            }
            let fname = f.get_function_name()?;
            match symbols.add_symbol(Symbol::Method(SymbolMethod::new(fname, class.clone())), Position::new(self.fq_file_name.clone(), f.lineno)) {
                Ok(noe) => eprintln!(" [ added ] {}", noe),
                Err(e) => {
                    let msg: String = e.to_string();
                    if msg == "UNIQUE constraint failed: symbol_method.class_id, symbol_method.name" {
                        eprintln!(" [  ok   ]");
                    } else {
                        eprintln!(" [ error ] {}", e)
                    }
                }
            }
            Ok("Ok")
        }
    */
}
