// extern crate rusqlite;


// use rusqlite::{Connection, Error};
use std::path::PathBuf;



#[derive(Clone, Debug)]
pub struct Position {
    file: PathBuf,
    lineno: u32,
}

impl Position {
    pub fn new(f: PathBuf, l: u32) -> Self {
        Position { file: f, lineno: l }
    }
}

/*impl Into<SymbolStorageError> for Error {
    fn into(self) -> SymbolStorageError {
        return SymbolStorageError::Sqlite(self);
    }
}*/
/*
impl From<Error> for SymbolStorageError {
    fn from(error: Error) -> Self {
        SymbolStorageError::Sqlite(error)
    }
}

pub struct SymbolStorage {
    db: Connection,
}

#[derive(Debug)]
pub enum SymbolStorageError {
    Sqlite(Error),
    Custom(&'static str),
}

*/
/*
impl SymbolStorage {
    pub fn new() -> std::result::Result<Self, SymbolStorageError> {
        let conn = Connection::open("phplint.db")?;
        SymbolStorage::init(&conn)?;
        Ok(SymbolStorage { db: conn })
    }

    fn init(conn: &Connection) -> Result<usize> {
        let mut changes: usize = 0;
        changes += conn.execute(
            "create table if not exists symbol_class (
                id integer primary key,
                name text not null,
                namespace text not null,
                parent text,
                filename text not null,
                lineno int,
                unique (name, namespace)
            )",
            [],
        )?;

        changes += conn.execute(
            "create table if not exists symbol_interface (
                id integer primary key,
                name text not null,
                namespace text not null,
                parent text,
                filename text not null,
                lineno int,
                unique (name, namespace)
            )",
            [],
        )?;

        changes += conn.execute(
            "create table if not exists symbol_trait (
                id integer primary key,
                name text not null,
                namespace text not null,
                parent text,
                filename text not null,
                lineno int,
                unique (name, namespace)
            )",
            [],
        )?;

        changes += conn.execute(
            "
            create table if not exists symbol_class_implements (
                class_id int not null references symbol_class(id),
                interface_id int not null references symbol_interface(id)
            )
            ",
            [],
        )?;

        changes += conn.execute(
            "
            create table if not exists symbol_class_traits (
                class_id int not null references symbol_class(id),
                interface_id int not null references symbol_trait(id)
            )
            ",
            [],
        )?;

        changes += conn.execute(
            "create table if not exists symbol_function (
                id integer primary key,
                name text not null unique
            )",
            [],
        )?;

        changes += conn.execute(
            "create table if not exists symbol_method (
                id integer primary key,
                name text not null,
                class_id int not null references symbol_class (id),
                unique (class_id, name)
            )",
            [],
        )?;

        changes += conn.execute(
            "create table if not exists method_parameter (
                id integer primary key,
                method_id int not null references symbol_method(id),
                name text not null,
                nullable bool,
                type text,
                default_val text,
                unique (method_id, name)
            )",
            [],
        )?;

        changes += conn.execute(
            "create table if not exists method_return (
                id integer primary key,
                method_id int not null references symbol_method(id),
                type text
            )",
            [],
        )?;

        Ok(changes)
    }

    pub fn add_symbol(&self, s: Symbol, p: Position) -> Result<usize> {
        let file: String = p
            .file
            .into_os_string()
            .into_string()
            .expect("Should be fine");
        match s {
            Symbol::Class(c) => {
                let name: String = c.name.to_string_lossy().to_string();
                let ns: String = c.ns.to_string_lossy().to_string();
                self
                .db
                .execute("INSERT INTO symbol_class (name, namespace, filename, lineno) VALUES (?1, ?2, ?3, ?4)", &[&name, &ns, &file, &p.lineno.to_string()])
            }
            Symbol::Method(m) => {
                let name = m.name.to_string_lossy().to_string();
                let cname = m.class.name.to_string_lossy().to_string();
                let ns = m.class.ns.to_string_lossy().to_string();

                self
                .db
                .execute("INSERT INTO symbol_method (name, class_id) VALUES (?1, (SELECT id FROM symbol_class WHERE name = ?2 AND namespace = ?3))",
                &[&name, &cname, &ns])
            }
            _ => {
                panic!(
                    "Vet ikke hvordan jeg skal arkivere et symbol på formen {:?}",
                    s
                );
            }
        }
    }

    pub fn get_symbol_id(&self, s: Symbol) -> Result<Option<i32>, SymbolStorageError> {
        match s {
            Symbol::Class(c) => {
                let mut stmt = self
                    .db
                    .prepare("select id from symbol_class where name = ?1")?;
                let name: String = c.name.to_string_lossy().to_string();
                let rows = stmt.query_map(&[&name], |row| row.get(0))?;
                for row in rows {
                    if let Ok(id) = row {
                        return Ok(Some(id));
                    }
                }
                return Ok(None);
            }
            Symbol::Method(m) => {
                let mut stmt = self.db.prepare(
                    "select
                                m.id
                            from
                                symbol_method m
                                    join symbol_class c on
                                        m.class_id = c.id
                            where
                                m.name = ?1 and
                                c.name = ?1",
                )?;
                let mname = m.name.to_string_lossy().to_string();
                let cname = m.class.name.to_string_lossy().to_string();
                let rows = stmt.query_map(&[&mname, &cname], |row| row.get(0))?;
                for row in rows {
                    if let Ok(id) = row {
                        return Ok(Some(id));
                    }
                }
                return Ok(None);
            }
            _ => Err(SymbolStorageError::Custom("Ukjent symboltype")),
        }
    }

    pub fn get_method_return_type(&self, method: SymbolMethod) -> Option<TypeOrigin> {
        eprintln!("@TODO, slå opp for å finne typen til {:?}", method);
        None
    }

    pub fn get_class_const_type(&self, constant: SymbolClassConstant) -> Option<TypeOrigin> {
        eprintln!("@TODO, slå opp for å finne typen til {:?}", constant);
        None
    }

    pub fn get_const_type(&self, constant: SymbolConstant) -> Option<TypeOrigin> {
        eprintln!("@TODO, slå opp for å finne typen til {:?}", constant);
        None
    }

    pub fn get_class_property_type(&self, property: SymbolClassProperty) -> Option<TypeOrigin> {
        eprintln!("@TODO, slå opp for å finne typen til {:?}", property);
        None
    }
}
*/
/*

fn main() -> Result<()> {
    let conn = Connection::open("cats.db")?;

    conn.execute(
        "create table if not exists cat_colors (
             id integer primary key,
             name text not null unique
         )",
        NO_PARAMS,
    )?;
    conn.execute(
        "create table if not exists cats (
             id integer primary key,
             name text not null,
             color_id integer not null references cat_colors(id)
         )",
        NO_PARAMS,
    )?;

    Ok(())
}
*/
