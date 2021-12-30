use std::convert::TryInto;

use php_tree_sitter::{
    analysis::state::AnalysisState, autonodes::any::AnyNodeRef, symboldata::ArcedSymbolAccess,
    symbols::Symbol,
};
use rust_lsp::{
    jsonrpc::MethodCompletable,
    lsp_types::{Location, Position, Range, TextDocumentPositionParams},
};
use url::Url;

use super::instance::PHPLanguageServerInstance;

pub fn goto_definition(
    phpls: &PHPLanguageServerInstance,
    position: TextDocumentPositionParams,
    completable: MethodCompletable<std::vec::Vec<Location>, ()>,
) {
    let result = phpls.at_position(
        position,
        Box::new(move |node, state, path| get_symbols_at_callback(node, state, path)),
    );

    let (symbol_data, symbols) = if let Ok((Some(symbol_data), Some(Some(symbols)))) = result {
        (symbol_data, symbols)
    } else {
        return;
    };

    let mut locations: Vec<_> = vec![];

    for symbol in symbols {
        for locs in symbol_data.get_pos_for_symbol(symbol) {
            for x in locs {
                let uri: Url = Url::parse(&x.uri.to_string_lossy().to_string())
                    .unwrap_or_else(|_| Url::parse("file://unknown_or_unparseable").unwrap());
                let range = Range {
                    start: Position {
                        line: x.start.line.try_into().unwrap(),
                        character: x.start.column.try_into().unwrap(),
                        // void
                    },
                    end: Position {
                        line: x.end.line.try_into().unwrap(),
                        character: x.end.column.try_into().unwrap(),
                        // void
                    },
                };
                locations.push(Location::new(uri, range))
            }
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
