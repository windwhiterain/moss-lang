use crate::language_server::LanguageServer;
use tower_lsp::{LspService, Server};

mod language_server;
#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| LanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
