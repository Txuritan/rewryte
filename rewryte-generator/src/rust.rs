use {
    crate::Error,
    rewryte_parser::models::{Enum, Item, Schema, Table, Types},
    std::io,
};

pub fn write_schema(schema: &Schema, writer: &mut impl io::Write) -> Result<(), Error> {
    for item in &schema.items {
        write_item(item, writer)?;
    }

    Ok(())
}

fn write_item(item: &Item, writer: &mut impl io::Write) -> Result<(), Error> {
    match &item {
        Item::Enum(decl) => write_enum(decl, writer)?,
        Item::Table(decl) => write_table(decl, writer)?,
    }

    Ok(())
}

fn write_enum(decl: &Enum, writer: &mut impl io::Write) -> Result<(), Error> {
    let ident = quote::format_ident!("{}", decl.name);

    let derive = if cfg!(feature = "serde") {
        quote::quote! {
            #[derive(serde::Deserialize, serde::Serialize)]
        }
    } else {
        quote::quote! {}
    };

    let variants = decl
        .variants
        .iter()
        .map(|v| quote::format_ident!("{}", v))
        .collect::<Vec<_>>();

    writeln!(
        writer,
        "{}",
        quote::quote! {
            #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
            #derive
            pub enum #ident {
                #( #variants, )*
            }
        }
    )?;

    #[cfg(feature = "postgres")]
    {
        use heck::KebabCase;

        let idents = std::iter::repeat(ident);
        let num_variants = decl.variants.len();

        let variants_kebab = decl
            .variants
            .iter()
            .map(|s| s.to_kebab_case())
            .collect::<Vec<String>>();

        {
            writeln!(
                writer,
                "{}",
                quote::quote! {
                    impl<'r> postgres_types::FromSql<'r> for #ident {
                        fn from_sql(_type: &postgres_types::Type, buf: &'a [u8]) -> std::result::Result<
                            #ident,
                            std::boxed::Box<dyn std::error::Error + std::marker::Sync + std::marker::Send>
                        > {
                            match std::str::from_utf8(buf)? {
                                #(
                                    #variants_kebab => std::result::Result::Ok(#idents::#variants),
                                )*
                                s => {
                                    std::result::Result::Err(
                                        std::convert::Into::into(format!("invalid variant `{}`", s))
                                    )
                                }
                            }
                        }

                        fn accepts(type_: &postgres_types::Type) -> bool {
                            if type_.name() != #name {
                                return false;
                            }

                            match *type_.kind() {
                                ::postgres_types::Kind::Enum(ref variants) => {
                                    if variants.len() != #num_variants {
                                        return false;
                                    }

                                    variants.iter().all(|v| {
                                        match &**v {
                                            #(
                                                #variant_names => true,
                                            )*
                                            _ => false,
                                        }
                                    })
                                }
                                _ => false,
                            }
                        }
                    }
                }
            )?;
        }
    }

    #[cfg(feature = "sqlite")]
    {
        use heck::KebabCase;

        let variants_kebab = decl
            .variants
            .iter()
            .map(|s| s.to_kebab_case())
            .collect::<Vec<String>>();

        {
            let idents = std::iter::repeat(ident.clone());

            writeln!(
                writer,
                "{}",
                quote::quote! {
                    impl rusqlite::types::ToSql for #ident {
                        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
                            match self {
                                #(
                                    #idents::#variants => std::result::Result::Ok(#variants_kebab.into()),
                                )*
                            }
                        }
                    }
                }
            )?;
        }

        {
            let idents = std::iter::repeat(ident.clone());

            writeln!(
                writer,
                "{}",
                quote::quote! {
                    impl rusqlite::types::FromSql for #ident {
                        fn column_result(value: rusqlite::types::ValueRef) -> rusqlite::types::FromSqlResult<Self> {
                            value.as_str().and_then(|s| match s {
                                #(
                                    #variants_kebab => Ok(#idents::#variants),
                                )*
                                _ => Err(rusqlite::types::FromSqlError::InvalidType),
                            })
                        }
                    }
                }
            )?;
        }
    }

    Ok(())
}

fn write_table(decl: &Table, writer: &mut impl io::Write) -> Result<(), Error> {
    let ident = quote::format_ident!("{}", decl.name);

    let derive = if cfg!(feature = "serde") {
        quote::quote! {
            #[derive(serde::Deserialize, serde::Serialize)]
        }
    } else {
        quote::quote! {}
    };

    let field_names = decl
        .columns
        .iter()
        .map(|c| c.name)
        .map(|c| quote::format_ident!("{}", c))
        .collect::<Vec<_>>();

    let field_types = decl
        .columns
        .iter()
        .map(|c| {
            (
                c,
                match c.typ {
                    Types::Char => quote::quote! { char },
                    Types::Varchar | Types::Text => quote::quote! { std::string::String },
                    Types::Number | Types::Int | Types::Serial | Types::MediumInt => {
                        quote::quote! { i32 }
                    }
                    Types::SmallInt => quote::quote! { i16 },
                    Types::BigInt => quote::quote! { i64 },
                    Types::Float | Types::Real | Types::Decimal => quote::quote! { f64 },
                    Types::Numeric => quote::quote! { f32 },
                    Types::DateTime => quote::quote! { chrono::DateTime<chrono::Utc> },
                    Types::Boolean => quote::quote! { bool },
                    Types::Raw(raw) => {
                        let raw_ident = quote::format_ident!("{}", raw);

                        quote::quote! { #raw_ident }
                    },
                },
            )
        })
        .map(|(c, t)| {
            if c.null {
                quote::quote! { std::option::Option<#t> }
            } else {
                t
            }
        })
        .collect::<Vec<_>>();

    writeln!(
        writer,
        "{}",
        quote::quote! {
            #[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
            #derive
            pub struct #ident {
                #(
                    pub #field_names: #field_types,
                )*
            }
        }
    )?;

    #[cfg(feature = "postgres")]
    {}

    #[cfg(feature = "sqlite")]
    {
        let ids = (0..(decl.columns.len())).map(|n| n).collect::<Vec<usize>>();
        let messages = ids
            .iter()
            .map(|n| format!("Failed to get data for row index {}", n))
            .collect::<Vec<_>>();

        writeln!(
            writer,
            "{}",
            quote::quote! {
                impl rewryte::sqlite::FromRow for #ident {
                    fn from_row(row: &rusqlite::Row<'_>) -> anyhow::Result<Self>
                    where
                        Self: Sized,
                    {
                        use anyhow::Context;

                        std::result::Result::Ok(Self {
                            #(
                                #field_names: row.get(#ids).context(#messages),
                            )*
                        })
                    }
                }
            }
        )?;
    }

    Ok(())
}
