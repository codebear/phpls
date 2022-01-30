use phpanalyzer::{
    analysis::state::AnalysisState,
    autonodes::any::AnyNodeRef,
    symboldata::{ArcedSymbolAccess, FileLocation},
    symbols::Symbol,
};
use rust_lsp::{
    jsonrpc::MethodCompletable,
    lsp_types::{
        request::{GotoDeclarationParams, GotoDeclarationResponse},
        Location, TextDocumentPositionParams,
    },
};

use crate::phpls::locations::file_locations_to_locations;

use super::instance::PHPLanguageServerInstance;

pub fn goto_declaration(
    phpls: &PHPLanguageServerInstance,
    params: GotoDeclarationParams,
    completable: MethodCompletable<GotoDeclarationResponse, ()>,
) {
    eprintln!("goto_declaration");

    let position = params.text_document_position_params;
    let locations = get_locations(phpls, position);
    eprintln!("Fant locations: {:?}", locations);
    completable.complete(Ok(GotoDeclarationResponse::Array(locations)))
}

fn get_locations(
    phpls: &PHPLanguageServerInstance,
    position: TextDocumentPositionParams,
) -> Vec<Location> {
    let pos_copy = position.clone();
    match phpls.at_position(
        position,
        Box::new(move |node, state, path| {
            // void
            eprintln!("AT POSITION: {:#?}", pos_copy.position);
            for node in path.iter().rev() {
                if let Some(file_locations) = get_file_location_for_node(node, state) {
                    return file_locations_to_locations(file_locations);
                } else if abort_upwords_traverse(node) {
                    return vec![];
                }
                eprintln!("PATH: {:?}", node.kind());
            }
            eprintln!("NODE: {:?}", node.kind());
            vec![]
        }),
    ) {
        Ok((_maybe_symbols, Some(locations))) => {
            // void
            locations
        }
        Ok((_, None)) => vec![],
        Err(e) => {
            eprintln!("ERROR: {}", e);
            vec![]
        }
    }
}

fn abort_upwords_traverse(node: &AnyNodeRef) -> bool {
    match node {
        AnyNodeRef::MethodDeclaration(_) => true,
        _ => false,
    }
}

fn get_file_location_for_node(
    node: &AnyNodeRef,
    state: &mut AnalysisState,
) -> Option<Vec<FileLocation>> {
    match node {
        AnyNodeRef::ScopedCallExpression(sc) => {
            if let Some(symbol) = sc.get_method_symbol(state) {

                if let Some(locations) = state.symbol_data.get_pos_for_symbol(Symbol::Method(symbol.clone())) {
                    Some(locations)
                } else {
                    eprintln!("Fant ikke noe posisjon til {:?}", symbol);
                    Some(vec![])
                }
            } else {
                eprintln!("Fant ikke noe symbol for metoden kalt av {:?}", sc);
                Some(vec![])
            }
        }
        AnyNodeRef::MemberCallExpression(mc) => {
            if let Some(symbols) = mc.get_symbols(state) {
                let mut locations = vec![];
                for symbol in symbols {

                    if let Some(locs) = state.symbol_data.get_pos_for_symbol(symbol.clone()) {
                        locations.extend(locs);
                    } else {
                        eprintln!("Fant ikke noe posisjon til {:?}", symbol);
                    }
                }
                Some(locations)
            } else {
                eprintln!("Fant ikke noen symboler for metoden kalt av {:?}", mc);
                Some(vec![])
            }

        }

        _ => None,
    }
}
