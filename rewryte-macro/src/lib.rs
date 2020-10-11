extern crate proc_macro;

use {
    codespan_reporting::{
        files::SimpleFiles,
        term::{self, termcolor::NoColor, Config},
    },
    proc_macro::TokenStream,
    rewryte_generator::{Format, FormatType},
    rewryte_parser::parser::{parse, Context},
    std::{
        fs,
        io::{BufWriter, ErrorKind},
        path::PathBuf,
    },
    syn::{
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        token::Comma,
        LitStr, Result, Token,
    },
};

fn error(path: LitStr, msg: impl std::fmt::Display) -> TokenStream {
    TokenStream::from(syn::Error::new_spanned(path, msg).to_compile_error())
}

#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    let input = match syn::parse::<FormatInput>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let contents = match fs::read_to_string(&input.path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return error(
                input.lit_path,
                format!("File does not exist: {}", input.path.display()),
            );
        }
        Err(err) => {
            return error(input.lit_path, err);
        }
    };

    let contents_str = contents.as_str();

    let mut files = SimpleFiles::new();

    let file_id = files.add("<inline>", contents_str);

    let mut ctx = Context::new(file_id);

    match parse(&mut ctx, contents_str) {
        Ok(schema) => {
            let mut writer = BufWriter::new(Vec::new());

            if let Err(err) = schema.fmt(&mut writer, input.format) {
                return error(input.lit_path, err);
            }

            let inner = match writer.into_inner() {
                Ok(vec) => vec,
                Err(err) => {
                    return error(input.lit_path, err);
                }
            };

            let rendered = match String::from_utf8(inner) {
                Ok(string) => string,
                Err(err) => {
                    return error(input.lit_path, err);
                }
            };

            TokenStream::from(quote::quote! {
                #rendered
            })
        }
        Err(err) => {
            let config = Config::default();

            let mut writer = NoColor::new(Vec::new());

            for diag in ctx.diagnostics() {
                if let Err(err) = term::emit(&mut writer, &config, &files, diag) {
                    return error(input.lit_path, err);
                }
            }

            let emit_string = match String::from_utf8(writer.into_inner()) {
                Ok(string) => string,
                Err(err) => {
                    return error(input.lit_path, err);
                }
            };

            TokenStream::from(
                syn::Error::new_spanned(input.lit_path, format!("{}\n\n{}", err, emit_string))
                    .to_compile_error(),
            )
        }
    }
}

struct FormatInput {
    format: FormatType,
    lit_path: LitStr,
    path: PathBuf,
}

impl Parse for FormatInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let lit_format = <LitStr as Parse>::parse(input)?;

        let format = match lit_format.value().as_str() {
            "mysql" => FormatType::MySQL,
            "postgresql" => FormatType::PostgreSQL,
            "sqlite" => FormatType::SQLite,
            "rust" => FormatType::Rust,
            _ => {
                return Err(syn::Error::new_spanned(
                    lit_format,
                    "Only the values `mysql`, `postgresql`, `sqlite`, and `rust` are allowed",
                ))
            }
        };

        let _ = input.parse::<Token![,]>()?;

        let lit_path = <LitStr as Parse>::parse(input)?;

        let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();

        let path = PathBuf::from(crate_root).join(lit_path.value());

        Ok(FormatInput {
            format,
            lit_path,
            path,
        })
    }
}

#[proc_macro]
pub fn models(input: TokenStream) -> TokenStream {
    let input = match syn::parse::<ModelInput>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let contents = match fs::read_to_string(&input.path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return error(
                input.lit_path,
                format!("File does not exist: {}", input.path.display()),
            );
        }
        Err(err) => {
            return error(input.lit_path, err);
        }
    };

    let contents_str = contents.as_str();

    let mut files = SimpleFiles::new();

    let file_id = files.add("<inline>", contents_str);

    let mut ctx = Context::new(file_id);

    match parse(&mut ctx, contents_str) {
        Ok(schema) => {
            let mut writer = BufWriter::new(Vec::new());

            let mut options = rewryte_generator::rust::Options::default();

            if let Some(extra) = input.extra {
                let mut mapped = extra.iter().map(LitStr::value);

                if mapped.by_ref().any(|value| &*value == "juniper") {
                    options.juniper = true;
                }

                if mapped.by_ref().any(|value| &*value == "serde") {
                    options.serde = true;
                }
            }

            if let Err(err) = rewryte_generator::rust::write_schema(&schema, &mut writer, options) {
                return error(input.lit_path, err);
            }

            let inner = match writer.into_inner() {
                Ok(vec) => vec,
                Err(err) => {
                    return error(input.lit_path, err);
                }
            };

            let rendered = match String::from_utf8(inner) {
                Ok(string) => string,
                Err(err) => {
                    return error(input.lit_path, err);
                }
            };

            match rendered.parse() {
                Ok(stream) => stream,
                Err(err) => error(input.lit_path, err),
            }
        }
        Err(err) => {
            let config = Config::default();

            let mut writer = NoColor::new(Vec::new());

            for diag in ctx.diagnostics() {
                if let Err(err) = term::emit(&mut writer, &config, &files, diag) {
                    return error(input.lit_path, err);
                }
            }

            let emit_string = match String::from_utf8(writer.into_inner()) {
                Ok(string) => string,
                Err(err) => {
                    return error(input.lit_path, err);
                }
            };

            error(input.lit_path, format!("{}\n\n{}", err, emit_string))
        }
    }
}

struct ModelInput {
    lit_path: LitStr,
    path: PathBuf,
    extra: Option<Vec<LitStr>>,
}

impl Parse for ModelInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let lit_path = <LitStr as Parse>::parse(input)?;

        let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();

        let path = PathBuf::from(crate_root).join(lit_path.value());

        let extra = if input.peek(syn::token::Comma) {
            let _comma = <Comma as Parse>::parse(input)?;

            if input.peek(syn::token::Bracket) {
                let content;

                let _bracket = syn::bracketed!(content in input);

                let parsed = Punctuated::<LitStr, Comma>::parse_terminated(&content)?;

                let mut items = Vec::with_capacity(parsed.len());

                for item in parsed {
                    items.push(item);
                }

                Some(items)
            } else {
                None
            }
        } else {
            None
        };

        Ok(ModelInput {
            lit_path,
            path,
            extra,
        })
    }
}
