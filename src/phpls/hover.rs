use php_tree_sitter::description::NodeDescription;
use rust_lsp::{
    jsonrpc::MethodCompletable,
    lsp_types::{
        Hover, HoverContents, MarkupContent, MarkupKind, Range, TextDocumentPositionParams,
    },
};

use super::instance::PHPLanguageServerInstance;

pub fn hover(
    phpls: &PHPLanguageServerInstance,
    params: TextDocumentPositionParams,
    completable: MethodCompletable<Hover, ()>,
) {
    eprintln!("hover(..): params: {:?}", params);
    let (markdown, range) = match get_hover_text(phpls, params) {
        Some(result) => result,
        None => ("".to_string(), None),
    };

    completable.complete(Ok(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown.clone(),
        }),
        range: range,
    }));
    // eprintln!("hover(..): range:{:?} complete markdown: {}", range, markdown);
}

fn get_hover_text(
    phpls: &PHPLanguageServerInstance,
    position: TextDocumentPositionParams,
) -> Option<(String, Option<Range>)> {
    let maybe_desc_result = phpls.at_position(
        position,
        Box::new(move |node, state, path| {
            eprintln!("foobar");
            let mut path = path.clone();
            path.push(node.clone());
            node.description(Some(&path[..]), state)
        }),
    );

    let desc = match maybe_desc_result {
        Ok((_, Some(Some(desc)))) => Some(desc),
        Err(e) => Some(format!("ERROR: {}", e)),
        _ => {
            // eprintln!("Unknown!?!?");
            None
        }
    }?;

    Some((desc, None))
}
