#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(all(feature = "sqlite"))]
pub mod sqlite;

pub use rewryte_macro::{models, schema};

#[cfg(feature = "build-script")]
use {
    codespan_reporting::{
        files::SimpleFiles,
        term::{self, termcolor::NoColor, Config},
    },
    rewryte_parser::parser::{parse, Context},
    std::{
        fs,
        io::{ErrorKind, Write},
        path::Path,
    },
};

#[cfg(feature = "build-script")]
pub fn models_to_writer<W, S>(writer: &mut W, schema: S, extra: Option<&[&str]>)
where
    W: Write,
    S: AsRef<Path>,
{
    let path: &Path = schema.as_ref();

    let contents = match fs::read_to_string(&path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            panic!("File does not exist: {}", path.display());
        }
        Err(err) => {
            panic!("{}: {:?}", path.display(), err);
        }
    };

    let contents_str = contents.as_str();

    let mut files = SimpleFiles::new();

    let file_id = files.add(path.display().to_string(), contents_str);

    let mut ctx = Context::new(file_id);

    match parse(&mut ctx, contents_str) {
        Ok(schema) => {
            let mut options = rewryte_generator::rust::Options::default();

            if let Some(extra) = extra {
                let mut mapped = extra.iter();

                if mapped.by_ref().any(|value| *value == "juniper") {
                    options.juniper = true;
                }

                if mapped.by_ref().any(|value| *value == "serde") {
                    options.serde = true;
                }
            }

            if let Err(err) = rewryte_generator::rust::write_schema(&schema, writer, options) {
                panic!("{}: {:?}", path.display(), err);
            }
        }
        Err(err) => {
            let config = Config::default();

            let mut writer = NoColor::new(Vec::new());

            for diag in ctx.diagnostics() {
                if let Err(err) = term::emit(&mut writer, &config, &files, diag) {
                    panic!("{}: {:?}", path.display(), err);
                }
            }

            let emit_string = match String::from_utf8(writer.into_inner()) {
                Ok(string) => string,
                Err(err) => {
                    panic!("{}: {:?}", path.display(), err);
                }
            };

            panic!("{}\n\n{}", err, emit_string)
        }
    }
}