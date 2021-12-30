
use crate::type_analysis::types::type_defs::TypeDefinition;
use crate::type_analysis::types::type_defs::PType;

#[derive(Debug, Clone)]
pub enum TypeOrigin {
    Implicit(TypeDefinition),
    Declared(TypeDefinition),
    Inferred(TypeDefinition),
    // Deferred(Symbol),
}

impl TypeOrigin {
    pub fn extend(&mut self, other: &Self) -> Self {
        *self = match self {
            TypeOrigin::Inferred(a) => match other {
                TypeOrigin::Inferred(b) | 
                TypeOrigin::Declared(b) | 
                TypeOrigin::Implicit(b) => TypeOrigin::Inferred(a.extend(b)),
            },
            TypeOrigin::Declared(a) => match other {
                TypeOrigin::Inferred(b) => TypeOrigin::Inferred(a.extend(b)), 
                TypeOrigin::Declared(b) | 
                TypeOrigin::Implicit(b) => TypeOrigin::Declared(a.extend(b)),
            },
            TypeOrigin::Implicit(a) => match other {
                TypeOrigin::Inferred(b) => TypeOrigin::Inferred(a.extend(b)),
                TypeOrigin::Declared(b) => TypeOrigin::Declared(a.extend(b)), 
                TypeOrigin::Implicit(b) => TypeOrigin::Implicit(a.extend(b)),
            },
        };
        self.clone()
    }

    pub fn reduce(types: Vec<Self>) -> Self {
        let mut it = types.iter();
        let mut type_origin = match it.next() {
            None => {
                return TypeOrigin::Inferred(PType::Unknown.as_def());
            },
            Some(f) => f.clone()
        };
        while let Some(next) = it.next() {
            type_origin.extend(next);
        }
        return type_origin.clone();
    }

    pub fn merge(a: &Self, b: &Self) -> Self {
        let mut t = a.clone();
        t.extend(b);
        t
    }

    pub fn as_def(&self) -> TypeDefinition {
        match &self {
            TypeOrigin::Inferred(a) |
            TypeOrigin::Declared(a) |
            TypeOrigin::Implicit(a) => a.clone()
        }
    }

    pub fn Unknown() -> Self {
        PType::Unknown.as_def().as_inferred()
    }
}