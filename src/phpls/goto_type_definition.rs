use std::convert::TryInto;

use php_tree_sitter::{
    autonodes::any::AnyNodeRef, issue::VoidEmitter, symboldata::ArcedSymbolAccess, symbols::Symbol,
    types::union::DiscreteType,
};
use rust_lsp::{
    jsonrpc::MethodCompletable,
    lsp_types::{
        request::{GotoDeclarationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse},
        Location, Position, Range,
    },
};
use url::Url;

use super::instance::PHPLanguageServerInstance;

pub fn goto_type_definition(
    phpls: &PHPLanguageServerInstance,
    params: GotoTypeDefinitionParams,
    completable: MethodCompletable<GotoTypeDefinitionResponse, ()>,
) {
    eprintln!("goto_type_definition");
    let position = params.text_document_position_params;

    let result = phpls.at_position(
        position,
        Box::new(move |node, state, path| {
            // void

            let mut our_path = path.clone();
            our_path.push(node.clone());

            let emitter = &VoidEmitter::new();

            for i_node in path.iter().rev() {
                let maybe_type = match i_node {
                    AnyNodeRef::_Literal(l) => l.get_utype(state, emitter),
                    AnyNodeRef::_PrimaryExpression(p) => p.get_utype(state, emitter),
                    AnyNodeRef::_Statement(s) => s.get_utype(state, emitter),
                    AnyNodeRef::_Type(t) => t.get_utype(state, emitter),

                    AnyNodeRef::Argument(a) => a.get_utype(state, emitter),

                    AnyNodeRef::ArrayElementInitializer(a) => a.get_utype(state, emitter),

                    AnyNodeRef::AssignmentExpression(a) => a.get_utype(state, emitter),

                    AnyNodeRef::AugmentedAssignmentExpression(a) => a.get_utype(state, emitter),

                    AnyNodeRef::BinaryExpression(b) => b.get_utype(state, emitter),

                    AnyNodeRef::ClassConstantAccessExpression(c) => c.get_utype(state, emitter),

                    AnyNodeRef::CompoundStatement(c) => c.get_utype(state, emitter),
                    AnyNodeRef::ConditionalExpression(c) => c.get_utype(state, emitter),

                    AnyNodeRef::DynamicVariableName(d) => d.get_utype(state, emitter),

                    AnyNodeRef::ExpressionStatement(e) => e.get_utype(state, emitter),

                    AnyNodeRef::MemberAccessExpression(m) => m.get_utype(state, emitter),
                    AnyNodeRef::MemberCallExpression(m) => m.get_utype(state, emitter),

                    AnyNodeRef::Name(n) => n.get_utype(state, emitter),

                    AnyNodeRef::NullsafeMemberAccessExpression(n) => n.get_utype(state, emitter),
                    AnyNodeRef::NullsafeMemberCallExpression(n) => n.get_utype(state, emitter),
                    AnyNodeRef::ObjectCreationExpression(oc) => oc.get_utype(state, emitter),
                    AnyNodeRef::OptionalType(ot) => ot.get_utype(state, emitter),

                    AnyNodeRef::PropertyDeclaration(pd) => pd.get_utype(state, emitter),
                    AnyNodeRef::PropertyElement(pe) => pe.get_utype(state, emitter),

                    AnyNodeRef::QualifiedName(qn) => qn.get_utype(state, emitter),

                    AnyNodeRef::ReturnStatement(r) => r.get_utype(state, emitter),
                    AnyNodeRef::ScopedCallExpression(s) => s.get_utype(state, emitter),
                    AnyNodeRef::ScopedPropertyAccessExpression(s) => s.get_utype(state, emitter),

                    AnyNodeRef::SimpleParameter(s) => s.get_utype(state, emitter),

                    AnyNodeRef::StaticVariableDeclaration(_s) => None,
                    AnyNodeRef::SubscriptExpression(s) => s.get_utype(state, emitter),

                    AnyNodeRef::VariableName(v) => v.get_utype(state, emitter),

                    AnyNodeRef::BaseClause(_) => {
                        // extends <Noe>
                        if let AnyNodeRef::Name(n) = &node {
                            let fq_base_name =
                                state.get_fq_symbol_name_from_local_name(&n.get_name());
                            Some(DiscreteType::Named(n.get_name(), fq_base_name).into())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(t) = maybe_type {
                    eprintln!("Fant type {:#?}", t);
                    return Some(t);
                }
            }
            return None;
        }),
    );
    // eprintln!("RESULT: {:?}", result);
    match result {
        Ok((Some(symbol_data), Some(Some(found_utype)))) => {
            let mut locations: Vec<_> = vec![];
            for t in found_utype.types {
                let symbol: Symbol = t.into();
                eprintln!("Looking for position for symbol: {:?}", symbol);

                if let Some(mut locs) = symbol_data.get_pos_for_symbol(symbol).map(|locs| {
                    locs.iter()
                        .map(|x| {
                            let mut in_uri = String::from("file://");
                            in_uri.push_str(&x.uri.to_string_lossy().to_string());
                            eprintln!("PRØVER Å URIFISERE {:?}", &x.uri);
                            let uri: Url = Url::parse(&in_uri).unwrap_or_else(|_| {
                                Url::parse("file://unknown_or_unparseable").unwrap()
                            });
                            eprintln!("BLE: {:?}", &uri);
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
                            Location::new(uri, range)
                        })
                        .collect::<Vec<_>>()
                }) {
                    locations.append(&mut locs);
                    // void
                }
            }
            eprintln!("Fant locatinos: {:?}", locations);
            completable.complete(Ok(GotoDeclarationResponse::Array(locations)))
        }
        Ok(_) => completable.complete(Ok(GotoDeclarationResponse::Array(vec![]))),
        Err(e) => {
            eprintln!("ERROR: {}", e);
            completable.complete(Ok(GotoDeclarationResponse::Array(vec![])))
        }
    }
}
