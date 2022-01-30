use phpanalyzer::{
    analysis::state::AnalysisState, autonodes::any::AnyNodeRef, symboldata::ArcedSymbolAccess,
    symbols::Symbol,
};
use rust_lsp::{
    jsonrpc::MethodCompletable,
    lsp_types::{Location, TextDocumentPositionParams},
};

use super::{instance::PHPLanguageServerInstance, locations::file_locations_to_locations};

pub fn goto_definition(
    phpls: &PHPLanguageServerInstance,
    position: TextDocumentPositionParams,
    completable: MethodCompletable<std::vec::Vec<Location>, ()>,
) {
    let result = phpls.at_position(
        position,
        Box::new(move |node, state, path| get_symbols_at_callback(node, state, path)),
    );
    let mut locations: Vec<_> = vec![];
    let (symbol_data, symbols) = if let Ok((Some(symbol_data), Some(Some(symbols)))) = result {
        (symbol_data, symbols)
    } else {
        completable.complete(Ok(locations));
        return;
    };

    for symbol in symbols {
        for file_locations in symbol_data.get_pos_for_symbol(symbol) {
            locations.extend(file_locations_to_locations(file_locations))
        }
    }

    completable.complete(Ok(locations));
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
