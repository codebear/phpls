

use crate::type_analysis::types::type_origins::TypeOrigin;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::hash::Hash;

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalarType {
    Int,
    String,
    Float,
    Num,
    Bool,
    ArrayKey,
}

impl ScalarType {
    pub fn as_ptype(&self) -> PType {
        PType::Scalar(self.clone())
    }

    pub fn as_def(&self) -> TypeDefinition {
        self.as_ptype().as_def()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PType {
    Scalar(ScalarType),
    Class(String, String),
    Vector(TypeDefinition),
    Shape(BTreeMap<String, TypeDefinition>),
    HashMap(ScalarType, TypeDefinition),
    Callable,
    Null,
    Void,
    Any,
    Unknown,
}

impl PType {
    pub fn as_def(&self) -> TypeDefinition {
        TypeDefinition::new_with_ptype(self.clone())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConcreteType {
    nullable: bool,
    ptype: PType,
}

impl ConcreteType {
    pub fn new(t: PType) -> Self {
        ConcreteType {
            nullable: false,
            ptype: t,
        }
    }

    pub fn new_nullable(t: PType) -> Self {
        ConcreteType {
            nullable: true,
            ptype: t,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeDefinition {
    types: BTreeSet<ConcreteType>,
}

impl TypeDefinition {
    pub fn new() -> Self {
        TypeDefinition {
            types: BTreeSet::new(),
        }
    }

    pub fn new_with_type(t: ConcreteType) -> Self {
        let mut x = Self::new();
        x.types.insert(t);
        x
    }
    pub fn new_with_ptype(t: PType) -> Self {
        Self::new_with_type(ConcreteType::new(t))
    }

    pub fn as_nullable(&self) -> Self {
        todo!();
    }

    pub fn extend(&mut self, other: &Self) -> Self {
        self.types.extend(other.clone().types);
        self.clone()
    }

    pub fn as_declared(&self) -> TypeOrigin {
        TypeOrigin::Declared(self.clone())
    }

    pub fn as_inferred(&self) -> TypeOrigin {
        TypeOrigin::Inferred(self.clone())
    }

    pub fn as_implicit(&self) -> TypeOrigin {
        TypeOrigin::Implicit(self.clone())
    }

    pub fn as_scalar(&self) -> Option<ScalarType> {
        let mut scalars = BTreeSet::new();
        for t in &self.types {
            match &t.ptype {
                PType::Scalar(s) => {
                    scalars.insert(s.clone());
                }
                _ => return None,
            }
        }
        if scalars.len() == 1 {
            return Some(scalars.iter().next().unwrap().clone());
        }
        if scalars.len() == 2
            && scalars.contains(&ScalarType::String)
            && scalars.contains(&ScalarType::Int)
        {
            return Some(ScalarType::ArrayKey);
        }
        if scalars.len() == 2
            && scalars.contains(&ScalarType::Int)
            && scalars.contains(&ScalarType::Float)
        {
            return Some(ScalarType::Num);
        }
        None
    }
    /*
    pub fn as_class_symbol(&self) -> Option<SymbolClass> {
        let mut classes = vec![];
        for t in &self.types {
            match &t.ptype {
                PType::Class(c, ns) => classes.push(SymbolClass::new(c.clone(), ns.clone())),
                _ => return None,
            }
        }
        SymbolClass::reduce_to_one(classes)
    }*/
}
