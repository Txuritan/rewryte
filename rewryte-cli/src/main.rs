use {
    codespan_reporting::{
        files::SimpleFiles,
        term::{
            self,
            termcolor::{ColorChoice, StandardStream},
            Config,
        },
    },
    rewryte_generator::{Format, FormatType},
    rewryte_parser::{parse, Context},
    std::{
        fs::{self, File},
        io::BufWriter,
        path::PathBuf,
    },
};

fn main() -> anyhow::Result<()> {
    let matches = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            clap::Arg::with_name("input")
                .long("input")
                .short("i")
                .value_name("FILE")
                .help("The required DAL schema file"),
        )
        .arg(
            clap::Arg::with_name("output")
                .long("output")
                .short("o")
                .value_name("FILE")
                .help("The file to write the transformed schema to")
                .conflicts_with("check"),
        )
        .arg(
            clap::Arg::with_name("format")
                .long("format")
                .short("f")
                .value_name("FORMAT")
                .takes_value(true)
                .possible_values(&["mysql", "postgres", "sqlite", "rust"])
                .help("What formats to export to")
                .conflicts_with("check"),
        )
        .arg(
            clap::Arg::with_name("check")
                .long("check")
                .short("c")
                .help("Checks the DAL schema file for syntax errors"),
        )
        .get_matches();

    let file = matches.value_of("input").unwrap();
    let path = PathBuf::from(file);
    let file_contents = fs::read_to_string(path)?;
    let contents_str = file_contents.as_str();

    let mut files = SimpleFiles::new();

    let file_id = files.add(file, contents_str);

    let mut ctx = Context::new(file_id);

    match parse(&mut ctx, contents_str) {
        Ok(schema) => {
            if !matches.is_present("check") {
                let typ = match matches.value_of("format") {
                    Some("mysql") => FormatType::MySQL,
                    Some("postgres") => FormatType::PostgreSQL,
                    Some("rust") => FormatType::Rust,
                    Some("sqlite") => FormatType::SQLite,
                    _ => unreachable!(),
                };

                let output = matches
                    .value_of("output")
                    .ok_or_else(|| anyhow::anyhow!("You must specify an output for the schema"))?;
                let file = File::create(output)?;
                let mut writer = BufWriter::new(file);

                schema.fmt(&mut writer, typ)?;
            }
        }
        Err(err) => {
            eprintln!("{:?}", err);

            let writer = StandardStream::stderr(ColorChoice::Always);
            let config = Config::default();

            for diag in ctx.diagnostics() {
                term::emit(&mut writer.lock(), &config, &files, diag)?;
            }
        }
    }

    Ok(())
}
