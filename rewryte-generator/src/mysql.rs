use {
    crate::Error,
    rewryte_parser::models::{Column, ColumnDefault, Enum, ForeignKey, Item, Schema, Table, Types},
    std::io,
};

pub fn write_schema(schema: &Schema, writer: &mut impl io::Write) -> Result<(), Error> {
    for item in &schema.items {
        write_item(item, writer)?;

        writeln!(writer)?;
    }

    Ok(())
}

pub fn write_item(item: &Item, writer: &mut impl io::Write) -> Result<(), Error> {
    match &item {
        Item::Enum(decl) => write_enum(decl, writer)?,
        Item::Table(decl) => write_table(decl, writer)?,
    }

    Ok(())
}

pub fn write_enum(_decl: &Enum, _writer: &mut impl io::Write) -> Result<(), Error> {
    todo!()
}

pub fn write_table(_decl: &Table, _writer: &mut impl io::Write) -> Result<(), Error> {
    todo!()
}

pub fn write_column(_column: &Column, _writer: &mut impl io::Write) -> Result<(), Error> {
    todo!()
}

pub fn write_types(_types: &Types, _writer: &mut impl io::Write) -> Result<(), Error> {
    todo!()
}

pub fn write_column_default(
    _column_default: &ColumnDefault,
    _writer: &mut impl io::Write,
) -> Result<(), Error> {
    todo!()
}

pub fn write_foreign_key(
    _foreign_key: &ForeignKey,
    _writer: &mut impl io::Write,
) -> Result<(), Error> {
    todo!()
}
