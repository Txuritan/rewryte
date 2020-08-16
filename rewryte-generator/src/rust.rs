use {
    crate::Error,
    codegen::Scope,
    rewryte_parser::models::{Enum, Item, Schema, Table, Types},
    std::{fmt::Write, io},
};

pub fn write_schema(schema: &Schema, writer: &mut impl io::Write) -> Result<(), Error> {
    let mut scope = Scope::new();

    for item in &schema.items {
        write_item(item, &mut scope)?;
    }

    write!(writer, "{}", scope.to_string())?;

    Ok(())
}

fn write_item(item: &Item, scope: &mut Scope) -> Result<(), Error> {
    match &item {
        Item::Enum(decl) => write_enum(decl, scope)?,
        Item::Table(decl) => write_table(decl, scope)?,
    }

    Ok(())
}

fn write_enum(decl: &Enum, scope: &mut Scope) -> Result<(), Error> {
    let item = scope.new_enum(decl.name).vis("pub")
        .derive("Clone")
        .derive("Debug")
        .derive("Hash")
        .derive("PartialEq").derive("Eq")
        .derive("PartialOrd").derive("Ord");

    #[cfg(feature = "serde")]
    {
        item.derive("serde::Deserialize")
            .derive("serde::Serialize");
    }

    for variant in &decl.variants {
        item.new_variant(variant);
    }

    Ok(())
}

fn write_table(decl: &Table, scope: &mut Scope) -> Result<(), Error> {
    let item = scope.new_struct(decl.name).vis("pub")
        .derive("Clone")
        .derive("Debug")
        .derive("Hash")
        .derive("PartialEq").derive("Eq")
        .derive("PartialOrd").derive("Ord");

    #[cfg(feature = "serde")]
    {
        item.derive("serde::Deserialize")
            .derive("serde::Serialize");
    }

    let mut buff = String::new();

    for column in &decl.columns {
        if column.null {
            write!(&mut buff, "std::option::Option<")?;
        }

        write!(
            &mut buff,
            "{}",
            match column.typ {
                Types::Char => "char",
                Types::Varchar | Types::Text => "String",
                Types::Number | Types::Int | Types::Serial | Types::MediumInt => "i32",
                Types::SmallInt => "i16",
                Types::BigInt => "i64",
                Types::Float | Types::Real | Types::Decimal => "f64",
                Types::Numeric => "REAL",
                Types::DateTime => "chrono::DateTime<chrono::Utc>",
                Types::Boolean => "bool",
                Types::Raw(raw) => raw,
            }
        )?;

        if column.null {
            write!(&mut buff, ">")?;
        }

        let name = format!("pub {}", column.name.to_lowercase());

        item.field(&name, buff.clone());

        buff.clear();
    }

    buff.clear();

    #[cfg(feature = "sqlite")]
    {
        scope.import("anyhow", "Context");

        let from_row = scope
            .new_impl(decl.name)
            .impl_trait("rewryte::sqlite::FromRow");

        let fun = from_row
            .new_fn("from_row")
            .arg("row", "&rusqlite::Row<'_>")
            .ret("anyhow::Result<Self>")
            .bound("Self", "Sized")
            .line("Ok(Self {");

        for (i, column) in decl.columns.iter().enumerate() {
            fun.line(format!(
                r#"{name}: row.get({id}).context("Failed to get data for row index {id}")?,"#,
                name = column.name.to_lowercase(),
                id = i,
            ));
        }

        fun.line("})");
    }

    Ok(())
}
