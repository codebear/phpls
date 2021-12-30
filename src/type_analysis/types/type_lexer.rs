#[derive(Debug)]
pub enum TypeLexItem {
    Nullable,
    Symbol(String),
    StartGeneric,
    EndGeneric,
    LeftParen,
    RightParen,
    LeftCurlyBracket,
    RightCurlyBracket,
    Colon,
    Comma,
    NamespaceSeparator,
}

impl TypeLexItem {
    pub fn lex(input: &String) -> Result<Vec<TypeLexItem>, String> {
        let mut result = Vec::new();
    
        let mut it = input.chars().peekable();
        while let Some(&c) = it.peek() {
            match c {
                '<' => {
                    it.next();
                    result.push(TypeLexItem::StartGeneric);
                },
                '>' => {
                    it.next();
                    result.push(TypeLexItem::EndGeneric);
                },
                '?' => {
                    it.next();
                    result.push(TypeLexItem::Nullable);
                },
                ':' => {
                    it.next();
                    result.push(TypeLexItem::Colon);
                },
                '{' => {
                    it.next();
                    result.push(TypeLexItem::LeftCurlyBracket);
                },
                '}' => {
                    it.next();
                    result.push(TypeLexItem::RightCurlyBracket);
                },
                '(' => {
                    it.next();
                    result.push(TypeLexItem::LeftParen);
                },
                ')' => {
                    it.next();
                    result.push(TypeLexItem::RightParen);
                },
                '\\' => {
                    it.next();
                    result.push(TypeLexItem::NamespaceSeparator);
                },
                ',' => {
                    it.next();
                    result.push(TypeLexItem::Comma);
                },
                'a'..='z'|'A'..='Z'|'_' => {
                    let mut symbol_chars = vec!(c);
                    it.next();
                    while let Some(&c2) = it.peek() {
                        match c2 {
                            'a'..='z'|'A'..='Z'|'_'|'0'..='9' => {
                                symbol_chars.push(c2);
                                it.next();
                            },
                            _ => break,
                        } 
                    }
                    result.push(TypeLexItem::Symbol(symbol_chars.iter().collect()));
                },
                _ => {
                    return Err(format!("unexpected character {}", c));
                }
            }
        }
        Ok(result)
    }
}