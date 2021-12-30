

use std::collections::BTreeMap;
use crate::type_analysis::types::type_defs::ConcreteType;
use crate::type_analysis::types::type_defs::PType;
use crate::type_analysis::types::type_defs::ScalarType;
use crate::type_analysis::types::type_defs::TypeDefinition;
use std::slice::Iter;
use std::iter::Peekable;

use crate::type_analysis::types::type_lexer::TypeLexItem;

trait ParseTokens {
    fn parse(&mut self, tokens: &mut Peekable<Iter<TypeLexItem>>) -> Result<Option<TypeDefinition>,String>;
}

#[derive(Debug)]
struct EmptyState { }

impl EmptyState {
    fn new() -> Self {
        EmptyState { }
    }
}

impl ParseTokens for EmptyState {
    fn parse(&mut self, tokens: &mut Peekable<Iter<TypeLexItem>>) -> Result<Option<TypeDefinition>,String> {
        let mut nullable = false;
        while let Some(token) = tokens.peek() {
            match (nullable, token) {
                (_, TypeLexItem::Symbol(s)) => {
                    if let Some(p_type) = match &s[..] {
                        "int" => Some(PType::Scalar(ScalarType::Int)),
                        "string" => Some(PType::Scalar(ScalarType::String)),
                        "float" => Some(PType::Scalar(ScalarType::Float)),
                        "bool" => Some(PType::Scalar(ScalarType::Bool)),
                        _ => None,
                    } {
                        tokens.next();
                        return Ok(Some(TypeDefinition::new_with_type(if nullable {
                            ConcreteType::new_nullable(p_type)
                        } else {
                            ConcreteType::new(p_type)
                        })));
                    }
                    match &s[..] {
                        "array" => return ArrayTypeState::new(nullable).parse(tokens),
                        _ => return ClassTypeState::new(nullable).parse(tokens),

                    }
                    // eprintln!("Mer tolking av typer {:?} token={:?}", self, token);
                    // todo!();
                    // return Err("Crap".to_string())
                },
                (false, TypeLexItem::Nullable) => {
                    tokens.next();
                    nullable = true;
                }
                _ => {
                    eprintln!("Fikk type-lex-ting: {:?}", token);
                    todo!("Det må implementeres mer komplekse typer");
                }
            }
        }
        todo!();
    }

}

#[derive(Clone)]
struct ArrayTypeState {
    nullable: bool,
}

impl ArrayTypeState {
    fn new(nullable: bool) -> Self {
        ArrayTypeState {
            nullable: nullable
        }
    }
}


impl ParseTokens for ArrayTypeState {
    fn parse(&mut self, tokens: &mut Peekable<Iter<TypeLexItem>>) -> Result<Option<TypeDefinition>,String> {
        if let Some(TypeLexItem::Symbol(t)) = tokens.peek() {
            if t == "array" {
                tokens.next();
            } else {
                return Err("Not an array-declaration".to_string());
            }
        } else {
            return Err("Not an array-declaration".to_string())
        }
        match &tokens.peek() {
            Some(TypeLexItem::LeftCurlyBracket) => {
                return ShapeTypeState::new().parse(tokens);
            }
            _ => {
                eprintln!("Neste token er {:?}", &tokens.peek());
            }
        }
        todo!();
    }
}

struct ClassTypeState {
    nullable: bool
}

impl ClassTypeState {
    fn new(nullable: bool) -> Self {
        ClassTypeState {
            nullable: nullable
        }
    }
}

impl ParseTokens for ClassTypeState {
    fn parse(&mut self, tokens: &mut Peekable<Iter<TypeLexItem>>) -> Result<Option<TypeDefinition>,String> {
        let first = if let Some(TypeLexItem::Symbol(next)) = tokens.next() {
            next
        } else {
            return Err("Fikk ikke symbol først".to_string());
        };
        let mut parts: Vec<String> = vec!(first.clone());
        let mut state = 0;
        loop {
            match (state, tokens.peek()) {
                (0, None) => break,
                (0, Some(TypeLexItem::NamespaceSeparator)) => {
                    state = 1;
                    tokens.next();
                },
                (1, Some(TypeLexItem::Symbol(s))) => {
                    state = 0;
                    tokens.next();
                    parts.push(s.clone());
                },
                _ => {
                    eprintln!("Neste token er : {:?}, state = {}", tokens.peek(), state);
                    todo!();
                }
            }
        }
        let last = match parts.pop() {
            None => return Err("Mangler symboler".to_string()),
            Some(p) => p
        };
        if parts.len() == 0 {
            return Ok(Some(TypeDefinition::new_with_type(if self.nullable {
                ConcreteType::new_nullable(PType::Class(last.to_string(), "".to_string()))
            } else {
                ConcreteType::new(PType::Class(last.to_string(), "".to_string()))
            })));
        }
        Ok(Some(TypeDefinition::new_with_type(if self.nullable {
            ConcreteType::new_nullable(PType::Class(last.to_string(), parts.join("\\")))
        } else {
            ConcreteType::new(PType::Class(last.to_string(), parts.join("\\")))
        })))
    }
}


struct ShapeTypeState {

}

impl ShapeTypeState {
    fn new() -> Self {
        ShapeTypeState {}
    }
}

impl ParseTokens for ShapeTypeState {
    fn parse(&mut self, tokens: &mut Peekable<Iter<TypeLexItem>>) -> Result<Option<TypeDefinition>,String> {
        if let Some(TypeLexItem::LeftCurlyBracket) = tokens.peek() {
            tokens.next();
        } else {
            return Err("Fant ikke left curly bracket".to_string());
        }
        let mut params = BTreeMap::<String, TypeDefinition>::new();
        let mut maybe_key = None;
        let mut state = 0; 
        loop {
            match (state, maybe_key, tokens.peek()) {
                (0, None, Some(TypeLexItem::Symbol(s))) => {
                    // Fant key
                    maybe_key = Some(s);
                    state = 1;
                    tokens.next();
                },
                (1, Some(key), Some(TypeLexItem::Colon)) => {
                    tokens.next();
                    if let Some(value_type) = EmptyState::new().parse(tokens)? {
                        params.insert(key.clone(), value_type);
                    } else {
                        return Err("Fant ikke type".to_string());
                    }
                    state = 2;
                    maybe_key = None;
                },
                (2, None, Some(TypeLexItem::Comma)) => {
                    tokens.next();
                    state = 0;
                },
                (0, None, Some(TypeLexItem::RightCurlyBracket)) |
                (2, None, Some(TypeLexItem::RightCurlyBracket)) => {
                        tokens.next();
                    break Ok(Some(PType::Shape(params).as_def()));
                },
                _ => {
                    eprintln!("Neste token er {:?}, state={}, key={:?}", &tokens.peek(), state, maybe_key);
                    todo!();
                }
            }
        }
    }
}
pub struct TypeParser {

}

impl TypeParser {
    pub fn new() -> Self {
        TypeParser { }
    }

    pub fn parse_string(&mut self, input: String) -> Result<Option<TypeDefinition>, String> {
        self.parse_vector(TypeLexItem::lex(&input)?)
    }

    fn parse_vector(&mut self, tokens: Vec<TypeLexItem>) -> Result<Option<TypeDefinition>, String> {
        EmptyState::new().parse(&mut tokens.iter().peekable())
    }
}

