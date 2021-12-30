use rust_lsp::{
    jsonrpc::MethodCompletable,
    lsp_types::request::{GotoDeclarationParams, GotoDeclarationResponse},
};

use super::instance::PHPLanguageServerInstance;

pub fn goto_declaration(
    phpls: &PHPLanguageServerInstance,
    params: GotoDeclarationParams,
    completable: MethodCompletable<GotoDeclarationResponse, ()>,
) {
    eprintln!("goto_declaration");

    let position = params.text_document_position_params;
    let pos_copy = position.clone();
    match phpls.at_position(
        position,
        Box::new(move |node, _state, path| {
            // void
            eprintln!("AT POSITION: {:#?}", pos_copy.position);
            for node in path.iter().rev() {
                eprintln!("PATH: {:?}", node.kind());
            }
            eprintln!("NODE: {:?}", node.kind())
        }),
    ) {
        Ok((_maybe_symbols, Some(_res))) => (),
        Ok((_, None)) => completable.complete(Ok(GotoDeclarationResponse::Array(vec![]))),
        Err(e) => {
            eprintln!("ERROR: {}", e);
            completable.complete(Ok(GotoDeclarationResponse::Array(vec![])))
        }
    }
}
