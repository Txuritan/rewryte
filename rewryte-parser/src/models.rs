use {
    crate::Error,
    std::{convert::TryFrom, fmt},
};

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Schema<'a> {
    pub items: Vec<Item<'a>>,
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Item<'a> {
    Enum(Enum<'a>),
    Table(Table<'a>),
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Enum<'a> {
    pub name: &'a str,
    pub not_exists: bool,
    pub variants: Vec<&'a str>,
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Table<'a> {
    pub name: &'a str,
    pub not_exists: bool,
    pub columns: Vec<Column<'a>>,
    pub primary_keys: Vec<&'a str>,
    pub foreign_keys: Vec<ForeignKey<'a>>,
    pub unique_keys: Vec<&'a str>,
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Column<'a> {
    pub name: &'a str,
    pub typ: Types<'a>,
    pub null: bool,
    pub default: ColumnDefault<'a>,
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Types<'a> {
    Boolean,

    // Text
    Char,
    Varchar,
    Text,

    // Numbers
    Number,
    SmallInt,
    MediumInt,
    BigInt,
    Int,
    Serial,

    // Floats
    Float,
    Real,
    Numeric,
    Decimal,

    // Date/Time
    DateTime,

    Raw(&'a str),
}

impl<'a> Types<'a> {
    pub(crate) fn from_str(s: &str) -> Types<'_> {
        match s {
            "bigInt" => Types::BigInt,
            "bool" | "boolean" => Types::Boolean,
            "char" => Types::Char,
            "dateTime" => Types::DateTime,
            "decimal" => Types::Decimal,
            "float" => Types::Float,
            "int" => Types::Int,
            "mediumInt" => Types::MediumInt,
            "number" => Types::Number,
            "numeric" => Types::Numeric,
            "real" => Types::Real,
            "serial" => Types::Serial,
            "smallInt" => Types::SmallInt,
            "text" => Types::Text,
            "varchar" => Types::Varchar,
            t => Types::Raw(t),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum ColumnDefault<'a> {
    None,
    Now,
    Null,
    Raw(&'a str),
}

impl<'a> Default for ColumnDefault<'a> {
    fn default() -> Self {
        ColumnDefault::None
    }
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct ForeignKey<'a> {
    pub local: &'a str,
    pub table: &'a str,
    pub foreign: &'a str,
    pub delete: Action,
    pub update: Action,
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub enum Action {
    NoAction,
    Restrict,
    SetNull,
    SetDefault,
    Cascade,
}

impl Default for Action {
    fn default() -> Self {
        Action::NoAction
    }
}

impl<'s> TryFrom<&'s str> for Action {
    type Error = Error;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        match value {
            "no action" => Ok(Action::NoAction),
            "restrict" => Ok(Action::Restrict),
            "set null" => Ok(Action::SetNull),
            "set default" => Ok(Action::SetDefault),
            "cascade" => Ok(Action::Cascade),
            t => Err(Error::InvalidAction(t.to_string())),
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Action::NoAction => "NO ACTION",
                Action::Restrict => "RESTRICT",
                Action::SetNull => "SET NULL",
                Action::SetDefault => "SET DEFAULT",
                Action::Cascade => "CASCADE",
            }
        )?;

        Ok(())
    }
}

pub(crate) struct ColumnPartial<'a> {
    pub name: &'a str,
    pub typ: Types<'a>,
    pub null: bool,
}

pub(crate) enum Modifier<'p> {
    Default {
        value: &'p str,
    },
    DefaultDateTime,
    DefaultNull,
    PrimaryKey,
    Reference {
        table: &'p str,
        column: &'p str,
        delete: Action,
        update: Action,
    },
    Unique,
}
