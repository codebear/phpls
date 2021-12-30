use crate::phpls::instance::PHPLanguageServerInstance;
use std::io::BufReader;
use std::net::TcpStream;
use std::net::TcpListener;
use std::thread;
use rust_lsp::lsp::LSPEndpoint;

pub struct PHPTCPLanguageServer {

}

impl PHPTCPLanguageServer {
    pub fn new() -> Self {
        PHPTCPLanguageServer {
            // void
        }
    }
    pub fn handle_connection(stream: TcpStream) {
        let out_stream = stream.try_clone().expect("Failed to clone stream");
        let endpoint = LSPEndpoint::create_lsp_output_with_output_stream(|| { out_stream });
        
        let ls = PHPLanguageServerInstance::new(endpoint.clone());
        
        let mut input = BufReader::new(stream);
        LSPEndpoint::run_server_from_input(&mut input, endpoint, ls);
    }

    pub fn run_listener(&self, listener: TcpListener) {
        let local_addr = listener.local_addr().unwrap();
        eprintln!("PHPLanguageServer.tcp_server: Listening to {}", local_addr);

        for stream in listener.incoming() {
            let stream = stream.expect("Failed to open incoming stream");
            let conn_handler = thread::spawn(move || {
                eprintln!("PHPLanguageServer.tcp_server in child_thread. Handling connection.");
                Self::handle_connection(stream)
            });
            
            // Only listen to first connection, so that this example can be run as a test
            conn_handler.join().unwrap();
            break; 
        }
        
        drop(listener);
    }
    
    pub fn start_tcp_server(&self) -> Result<(), String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let local_addr = listener.local_addr().unwrap();
        eprintln!("PHPLanguageServer.start: Bound to {}", local_addr);
        
        //let server_listener = thread::spawn(|| {
        self.run_listener(listener);
        //});
        Ok(())
        // let server = LSPEndpoint::asdf();
        // void
    }
}