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

// TODO: figure out how to handle `IF NOT EXISTS`
pub fn write_enum(decl: &Enum, writer: &mut impl io::Write) -> Result<(), Error> {
    write!(writer, "CREATE TYPE {} AS ENUM (", decl.name)?;

    writeln!(writer)?;

    for (i, variant) in decl.variants.iter().enumerate() {
        write!(writer, "  '{}'", variant)?;

        if i != decl.variants.len() - 1 {
            write!(writer, ",")?;
        }

        writeln!(writer)?;
    }

    write!(writer, ");")?;

    Ok(())
}

pub fn write_table(decl: &Table, writer: &mut impl io::Write) -> Result<(), Error> {
    write!(writer, "CREATE TABLE")?;

    if decl.not_exists {
        write!(writer, " IF NOT EXISTS")?;
    }

    write!(writer, " {} (", decl.name)?;

    writeln!(writer)?;

    for column in &decl.columns {
        write_column(column, writer)?;

        write!(writer, ",")?;

        writeln!(writer)?;
    }

    write!(writer, "  PRIMARY KEY (")?;

    for (i, primary) in decl.primary_keys.iter().enumerate() {
        write!(writer, "{}", primary)?;

        if i != decl.primary_keys.len() - 1 {
            write!(writer, ", ")?;
        }
    }

    write!(writer, ")")?;

    if !decl.foreign_keys.is_empty() {
        write!(writer, ",")?;
        writeln!(writer)?;

        for (i, foreign_key) in decl.foreign_keys.iter().enumerate() {
            write_foreign_key(foreign_key, writer)?;

            if i != decl.foreign_keys.len() - 1 {
                write!(writer, ",")?;

                writeln!(writer)?;
            }
        }

        if decl.unique_keys.is_empty() {
            writeln!(writer)?;
        }
    } else if decl.unique_keys.is_empty() {
        writeln!(writer)?;
    }

    if !decl.unique_keys.is_empty() {
        write!(writer, ",")?;
        writeln!(writer)?;

        write!(writer, "  UNIQUE (")?;

        for (i, unique) in decl.unique_keys.iter().enumerate() {
            write!(writer, "{}", unique)?;

            if i != decl.unique_keys.len() - 1 {
                write!(writer, ", ")?;
            }
        }

        write!(writer, ")")?;

        writeln!(writer)?;
    }

    write!(writer, ");")?;

    Ok(())
}

pub fn write_column(column: &Column, writer: &mut impl io::Write) -> Result<(), Error> {
    write!(writer, "  {} ", column.name,)?;

    write_types(&column.typ, writer)?;

    if !column.null {
        write!(writer, " NOT NULL")?;
    }

    write_column_default(&column.default, writer)?;

    Ok(())
}

pub fn write_types(types: &Types, writer: &mut impl io::Write) -> Result<(), Error> {
    write!(
        writer,
        "{}",
        match types {
            Types::Char => r#""char""#,
            Types::Text => "TEXT",
            Types::Varchar => "VARCHAR",
            Types::SmallInt => "SMALLINT",
            Types::Number | Types::Int | Types::MediumInt | Types::Serial => "INT",
            Types::BigInt => "BIGINT",
            Types::Float | Types::Real => "REAL",
            Types::Numeric => "NUMERIC",
            Types::Decimal => "DECIMAL",
            Types::DateTime => "TIMESTAMP WITH TIME ZONE",
            Types::Boolean => "BOOL",
            Types::Raw(raw) => raw,
        }
    )?;

    Ok(())
}

pub fn write_column_default(
    column_default: &ColumnDefault,
    writer: &mut impl io::Write,
) -> Result<(), Error> {
    if column_default != &ColumnDefault::None {
        write!(writer, " DEFAULT")?;

        match column_default {
            ColumnDefault::Now => {
                write!(writer, " (timezone('utc', now()))")?;
            }
            ColumnDefault::Null => {
                write!(writer, " NULL")?;
            }
            ColumnDefault::Raw(raw) => {
                write!(writer, " {}", raw)?;
            }
            ColumnDefault::None => unreachable!(),
        }
    }

    Ok(())
}

pub fn write_foreign_key(
    foreign_key: &ForeignKey,
    writer: &mut impl io::Write,
) -> Result<(), Error> {
    write!(
        writer,
        "  FOREIGN KEY ({}) REFERENCES {}({}) ON UPDATE {} ON DELETE {}",
        foreign_key.local,
        foreign_key.table,
        foreign_key.foreign,
        foreign_key.update,
        foreign_key.delete,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    mod enums {
        use {crate::postgresql::write_enum, rewryte_parser::models::*};

        #[test]
        fn simple() {
            let decl = Enum {
                name: "Test",
                not_exists: false,
                variants: vec!["Variant1", "Variant2"],
            };

            let mut writer = Vec::new();

            write_enum(&decl, &mut writer).expect("Unable to write enum to buffer");

            let utf8_writer =
                String::from_utf8(writer).expect("Unable to convert buff into string");

            assert_eq!(
                "CREATE TYPE Test AS ENUM (
  'Variant1',
  'Variant2'
);",
                utf8_writer.as_str(),
            );
        }
    }

    mod tables {
        use {crate::postgresql::write_table, rewryte_parser::models::*};

        #[test]
        fn simple() {
            let table = Table {
                name: "Example",
                not_exists: true,
                columns: vec![
                    Column {
                        name: "Id",
                        typ: Types::Text,
                        null: false,
                        default: ColumnDefault::None,
                    },
                    Column {
                        name: "Name",
                        typ: Types::Text,
                        null: false,
                        default: ColumnDefault::None,
                    },
                ],
                primary_keys: vec!["Id"],
                foreign_keys: vec![],
                unique_keys: vec![],
            };

            let mut buff = Vec::new();

            write_table(&table, &mut buff).expect("Unable to write table to buffer");

            let utf8_buff = String::from_utf8(buff).expect("Unable to convert buff into string");

            assert_eq!(
                "CREATE TABLE IF NOT EXISTS Example (
  Id TEXT NOT NULL,
  Name TEXT NOT NULL,
  PRIMARY KEY (Id)
);",
                utf8_buff.as_str()
            );
        }
    }
}
