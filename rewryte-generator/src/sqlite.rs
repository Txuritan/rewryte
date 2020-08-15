use {
    crate::Error,
    rewryte_parser::models::{Column, ColumnDefault, Enum, ForeignKey, Item, Schema, Table, Types},
    std::io,
};

pub fn write_schema(schema: &Schema, writer: &mut impl io::Write) -> Result<(), Error> {
    for (i, item) in schema.items.iter().enumerate() {
        write_item(item, writer)?;

        writeln!(writer)?;

        if i != schema.items.len() - 1 {
            writeln!(writer)?;
        }
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
    // TODO: maybe log a warning?
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
            Types::Char | Types::Text => "TEXT",
            Types::Varchar => "VARCHAR",
            Types::Number | Types::SmallInt | Types::MediumInt | Types::Int | Types::Serial => {
                "INTEGER"
            }
            Types::BigInt => "BIGINT",
            Types::Float | Types::Real | Types::Numeric => "REAL",
            Types::Decimal => "DECIMAL",
            Types::DateTime => "DATETIME",
            Types::Boolean => "BOOLEAN",
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
                write!(writer, " (DATETIME('now', 'utc'))")?;
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

// TODO: Maybe I can clean this up
#[cfg(test)]
mod tests {
    use {crate::sqlite::write_table, rewryte_parser::models::*};

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

    #[test]
    fn multiple_primary_keys() {
        let table = Table {
            name: "Example",
            not_exists: true,
            columns: vec![
                Column {
                    name: "Key",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::None,
                },
                Column {
                    name: "Value",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::None,
                },
            ],
            primary_keys: vec!["Key", "Value"],
            foreign_keys: vec![],
            unique_keys: vec![],
        };

        let mut buff = Vec::new();

        write_table(&table, &mut buff).expect("Unable to write table to buffer");

        let utf8_buff = String::from_utf8(buff).expect("Unable to convert buff into string");

        assert_eq!(
            "CREATE TABLE IF NOT EXISTS Example (
  Key TEXT NOT NULL,
  Value TEXT NOT NULL,
  PRIMARY KEY (Key, Value)
);",
            utf8_buff.as_str()
        );
    }

    #[test]
    fn foreign_keys() {
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
                Column {
                    name: "Other",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::None,
                },
            ],
            primary_keys: vec!["Id"],
            foreign_keys: vec![ForeignKey {
                local: "Other",
                table: "Other",
                foreign: "Id",
                delete: Action::default(),
                update: Action::default(),
            }],
            unique_keys: vec![],
        };

        let mut buff = Vec::new();

        write_table(&table, &mut buff).expect("Unable to write table to buffer");

        let utf8_buff = String::from_utf8(buff).expect("Unable to convert buff into string");

        assert_eq!(
            "CREATE TABLE IF NOT EXISTS Example (
  Id TEXT NOT NULL,
  Name TEXT NOT NULL,
  Other TEXT NOT NULL,
  PRIMARY KEY (Id),
  FOREIGN KEY (Other) REFERENCES Other(Id) ON UPDATE NO ACTION ON DELETE NO ACTION
);",
            utf8_buff.as_str()
        );
    }

    #[test]
    fn unique_keys() {
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
                    name: "Key",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::None,
                },
                Column {
                    name: "Value",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::None,
                },
            ],
            primary_keys: vec!["Id"],
            foreign_keys: vec![],
            unique_keys: vec!["Key"],
        };

        let mut buff = Vec::new();

        write_table(&table, &mut buff).expect("Unable to write table to buffer");

        let utf8_buff = String::from_utf8(buff).expect("Unable to convert buff into string");

        assert_eq!(
            "CREATE TABLE IF NOT EXISTS Example (
  Id TEXT NOT NULL,
  Key TEXT NOT NULL,
  Value TEXT NOT NULL,
  PRIMARY KEY (Id),
  UNIQUE (Key)
);",
            utf8_buff.as_str()
        );
    }

    #[test]
    fn unique_keys_foreign_keys() {
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
                Column {
                    name: "Other",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::None,
                },
            ],
            primary_keys: vec!["Id"],
            foreign_keys: vec![ForeignKey {
                local: "Other",
                table: "Other",
                foreign: "Id",
                delete: Action::default(),
                update: Action::default(),
            }],
            unique_keys: vec!["Name"],
        };

        let mut buff = Vec::new();

        write_table(&table, &mut buff).expect("Unable to write table to buffer");

        let utf8_buff = String::from_utf8(buff).expect("Unable to convert buff into string");

        assert_eq!(
            "CREATE TABLE IF NOT EXISTS Example (
  Id TEXT NOT NULL,
  Name TEXT NOT NULL,
  Other TEXT NOT NULL,
  PRIMARY KEY (Id),
  FOREIGN KEY (Other) REFERENCES Other(Id) ON UPDATE NO ACTION ON DELETE NO ACTION,
  UNIQUE (Name)
);",
            utf8_buff.as_str()
        );
    }
}
