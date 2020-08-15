pub mod mysql;
pub mod postgresql;
pub mod rust;
pub mod sqlite;

use {
    rewryte_parser::models::Schema,
    std::{convert::TryFrom, fmt, io},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("`{0}` is not a valid format type")]
    InvalidFormat(String),

    #[error("Format error")]
    Format(#[from] fmt::Error),
    #[error("IO error")]
    Io(#[from] io::Error),
}

#[derive(Clone, Copy, Debug)]
pub enum FormatType {
    MySQL,
    PostgreSQL,
    Rust,
    SQLite,
}

impl<'s> TryFrom<&'s str> for FormatType {
    type Error = Error;

    fn try_from(s: &'s str) -> Result<Self, Self::Error> {
        match s {
            "mysql" => Ok(FormatType::MySQL),
            "postgresql" => Ok(FormatType::PostgreSQL),
            "rust" => Ok(FormatType::Rust),
            "sqlite" => Ok(FormatType::SQLite),
            t => Err(Error::InvalidFormat(t.to_string())),
        }
    }
}

pub trait Format<W: io::Write> {
    fn fmt(&self, writer: &mut W, typ: FormatType) -> Result<(), Error>;
}

impl<'i, W: io::Write> Format<W> for Schema<'i> {
    fn fmt(&self, writer: &mut W, typ: FormatType) -> Result<(), Error> {
        match typ {
            FormatType::MySQL => mysql::write_schema(self, writer)?,
            FormatType::PostgreSQL => postgresql::write_schema(self, writer)?,
            FormatType::SQLite => sqlite::write_schema(self, writer)?,
            FormatType::Rust => todo!(),
        }

        Ok(())
    }
}
