use crate::phpls::instance::PHPLanguageServerInstance;
use rust_lsp::lsp::LSPEndpoint;
use std::io::stdin;
use std::io::stdout;
use std::io::BufReader;

pub struct PHPStdIOLanguageServer {}

impl PHPStdIOLanguageServer {
    pub fn new() -> Self {
        PHPStdIOLanguageServer {}
    }

    pub fn start_stdio_server(&self) -> Result<(), String> {
        let out_stream = stdout();
        let endpoint = LSPEndpoint::create_lsp_output_with_output_stream(|| out_stream);

        let server_handler = PHPLanguageServerInstance::new(endpoint.clone());

        let mut input = BufReader::new(stdin());
        LSPEndpoint::run_server_from_input(&mut input, endpoint, server_handler);
        Ok(())
    }
}
