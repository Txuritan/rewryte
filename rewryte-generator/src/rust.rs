use {
    crate::Error,
    heck::{KebabCase, SnakeCase},
    rewryte_parser::models::{Enum, Item, Schema, Table, Types},
    std::io,
};

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Options {
    pub juniper: bool,
    pub serde: bool,
}

pub fn write_schema(
    schema: &Schema,
    writer: &mut impl io::Write,
    options: Options,
) -> Result<(), Error> {
    for item in &schema.items {
        write_item(item, writer, options)?;
    }

    Ok(())
}

pub fn write_item(item: &Item, writer: &mut impl io::Write, options: Options) -> Result<(), Error> {
    match &item {
        Item::Enum(decl) => write_enum(decl, writer, options)?,
        Item::Table(decl) => write_table(decl, writer, options)?,
    }

    Ok(())
}

pub fn write_enum(decl: &Enum, writer: &mut impl io::Write, options: Options) -> Result<(), Error> {
    let ident = quote::format_ident!("{}", decl.name);

    let juniper_derive = if options.juniper {
        if cfg!(feature = "feature-gate-juniper") {
            quote::quote! {
                #[cfg_attr(feature = "rewryte-juniper", derive(juniper::GraphQLEnum))]
            }
        } else {
            quote::quote! {
                #[derive(juniper::GraphQLEnum)]
            }
        }
    } else {
        quote::quote! {}
    };

    let serde_derive = if options.serde {
        if cfg!(feature = "feature-gate-serde") {
            quote::quote! {
                #[cfg_attr(feature = "rewryte-serde", derive(serde::Deserialize, serde::Serialize))]
            }
        } else {
            quote::quote! {
                #[derive(serde::Deserialize, serde::Serialize)]
            }
        }
    } else {
        quote::quote! {}
    };

    let variants = decl
        .variants
        .iter()
        .map(|v| quote::format_ident!("{}", v))
        .collect::<Vec<_>>();

    let variants_rename = decl
        .variants
        .iter()
        .map(|v| {
            if options.serde {
                let kebab = v.to_kebab_case();

                quote::quote! { #[serde(rename = #kebab)] }
            } else {
                quote::quote! {}
            }
        })
        .collect::<Vec<_>>();

    writeln!(
        writer,
        "{}",
        quote::quote! {
            #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
            #juniper_derive
            #serde_derive
            pub enum #ident {
                #(
                    #variants_rename
                    #variants,
                )*
            }
        }
    )?;

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    {
        let variants_kebab = decl
            .variants
            .iter()
            .map(|s| s.to_kebab_case())
            .collect::<Vec<String>>();

        #[cfg(feature = "postgres")]
        {
            let name = decl.name;
            let idents = std::iter::repeat(ident.clone());
            let num_variants = decl.variants.len();

            let variant_names = &decl.variants;

            {
                writeln!(
                    writer,
                    "{}",
                    quote::quote! {
                        impl<'r> ::rewryte::postgres::types::FromSql<'r> for #ident {
                            fn from_sql(_type: &::rewryte::postgres::types::Type, buf: &'r [u8]) -> ::std::result::Result<
                                #ident,
                                ::std::boxed::Box<dyn ::std::error::Error + ::std::marker::Sync + ::std::marker::Send>
                            > {
                                match ::std::str::from_utf8(buf)? {
                                    #(
                                        #variants_kebab => ::std::result::Result::Ok(#idents::#variants),
                                    )*
                                    s => {
                                        ::std::result::Result::Err(
                                            ::std::convert::Into::into(format!("invalid variant `{}`", s))
                                        )
                                    }
                                }
                            }

                            fn accepts(type_: &::rewryte::postgres::types::Type) -> bool {
                                if type_.name() != #name {
                                    return false;
                                }

                                match *type_.kind() {
                                    ::rewryte::postgres::types::Kind::Enum(ref variants) => {
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
            {
                let idents = std::iter::repeat(ident.clone());

                writeln!(
                    writer,
                    "{}",
                    quote::quote! {
                        impl ::rewryte::sqlite::types::ToSql for #ident {
                            fn to_sql(&self) -> ::rewryte::sqlite::Result<::rewryte::sqlite::types::ToSqlOutput> {
                                match self {
                                    #(
                                        #idents::#variants => ::std::result::Result::Ok(#variants_kebab.into()),
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
                        impl ::rewryte::sqlite::types::FromSql for #ident {
                            fn column_result(value: ::rewryte::sqlite::types::ValueRef) -> ::rewryte::sqlite::types::FromSqlResult<Self> {
                                value.as_str().and_then(|s| match s {
                                    #(
                                        #variants_kebab => ::std::result::Result::Ok(#idents::#variants),
                                    )*
                                    _ => ::std::result::Result::Err(::rewryte::sqlite::types::FromSqlError::InvalidType),
                                })
                            }
                        }
                    }
                )?;
            }
        }
    }

    Ok(())
}

pub fn write_table(
    decl: &Table,
    writer: &mut impl io::Write,
    options: Options,
) -> Result<(), Error> {
    let ident = quote::format_ident!("{}", decl.name);

    let juniper_derive = if options.juniper {
        quote::quote! {
            #[derive(juniper::GraphQLObject)]
        }
    } else {
        quote::quote! {}
    };

    let serde_derive = if options.serde {
        quote::quote! {
            #[derive(serde::Deserialize, serde::Serialize)]
        }
    } else {
        quote::quote! {}
    };

    let field_names = decl
        .columns
        .iter()
        .map(|c| quote::format_ident!("{}", c.name.to_snake_case()))
        .collect::<Vec<_>>();

    let field_types = decl
        .columns
        .iter()
        .map(|c| {
            (
                c.null,
                match c.typ {
                    Types::Char => quote::quote! { char },
                    Types::Varchar | Types::Text => quote::quote! { ::std::string::String },
                    Types::Number | Types::Int | Types::Serial | Types::MediumInt => {
                        quote::quote! { i32 }
                    }
                    Types::SmallInt => quote::quote! { i16 },
                    Types::BigInt => quote::quote! { i64 },
                    Types::Float | Types::Real | Types::Decimal => quote::quote! { f64 },
                    Types::Numeric => quote::quote! { f32 },
                    Types::DateTime => quote::quote! { ::chrono::DateTime<chrono::Utc> },
                    Types::Boolean => quote::quote! { bool },
                    Types::Raw(raw) => {
                        let raw_ident = quote::format_ident!("{}", raw);

                        quote::quote! { #raw_ident }
                    }
                },
            )
        })
        .map(|(null, t)| {
            if null {
                quote::quote! { ::std::option::Option<#t> }
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
            #juniper_derive
            #serde_derive
            pub struct #ident {
                #(
                    pub #field_names: #field_types,
                )*
            }
        }
    )?;

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    {
        let ids = (0..(decl.columns.len())).map(|n| n).collect::<Vec<usize>>();
        let messages = ids
            .iter()
            .map(|n| {
                format!(
                    "Failed to get data for row index {}: `{}`",
                    n,
                    decl.columns[*n].name.to_snake_case()
                )
            })
            .collect::<Vec<_>>();

        #[cfg(feature = "postgres")]
        {
            writeln!(
                writer,
                "{}",
                quote::quote! {
                    impl ::rewryte::postgres::FromRow for #ident {
                        fn from_row(row: ::rewryte::postgres::Row) -> ::anyhow::Result<Self>
                        where
                            Self: Sized,
                        {
                            use ::anyhow::Context;

                            ::std::result::Result::Ok(Self {
                                #(
                                    #field_names: row.try_get(#ids).context(#messages)?,
                                )*
                            })
                        }
                    }
                }
            )?;
        }

        #[cfg(feature = "sqlite")]
        {
            writeln!(
                writer,
                "{}",
                quote::quote! {
                    impl ::rewryte::sqlite::FromRow for #ident {
                        fn from_row(row: &::rewryte::sqlite::Row<'_>) -> ::anyhow::Result<Self>
                        where
                            Self: Sized,
                        {
                            use ::anyhow::Context;

                            ::std::result::Result::Ok(Self {
                                #(
                                    #field_names: row.get(#ids).context(#messages)?,
                                )*
                            })
                        }
                    }
                }
            )?;
        }
    }

    Ok(())
}
