use {
    crate::{
        models::{
            Action, Column, ColumnDefault, ColumnPartial, Enum, ForeignKey, Item, Modifier, Schema,
            Table, Types,
        },
        Error,
    },
    codespan_reporting::diagnostic::{Diagnostic, Label},
    pest::{
        iterators::{Pair, Pairs},
        Parser as _, Span,
    },
    std::{convert::TryFrom, ops::Range},
};

#[derive(pest_derive::Parser)]
#[grammar = "dal.pest"]
struct Parser;

pub struct Context {
    pub(crate) diags: Vec<Diagnostic<usize>>,
    file_id: usize,
}

impl Context {
    pub fn new(file_id: usize) -> Self {
        Self {
            diags: Vec::new(),
            file_id,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic<usize>] {
        &self.diags
    }
}

#[inline]
fn span_range_end(span: Span) -> Range<usize> {
    (span.end())..(span.end())
}

#[inline]
fn span_range_single<'i>(pair: &Pair<'i, Rule>) -> Range<usize> {
    let span = pair.as_span();

    (span.start())..(span.start())
}

#[allow(dead_code)]
pub fn parse<'i>(ctx: &mut Context, input: &'i str) -> Result<Schema<'i>, Error> {
    let mut pairs: Pairs<'i, Rule> = Parser::parse(Rule::schema, input)?;

    let mut items = Vec::new();

    let pair = match pairs.next() {
        Some(pair) if pair.as_rule() == Rule::schema => pair,
        Some(pair) if pair.as_rule() == Rule::EOI => return Ok(Schema { items }),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `schema`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            return Err(Error::UnexpectedEOS);
        }
    };

    for root_group in pair.into_inner() {
        match root_group.as_rule() {
            Rule::decl_enum => {
                let decl = parse_enum(ctx, root_group)?;

                items.push(Item::Enum(decl));
            }
            Rule::decl_table => {
                let decl = parse_table(ctx, root_group)?;

                items.push(Item::Table(decl));
            }
            Rule::comment => continue,
            Rule::EOI => break,
            _ => {
                ctx.diags.push(
                    Diagnostic::error()
                        .with_message("Unexpected token")
                        .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&root_group))
                            .with_message(format!(
                                "expected `enum declaration`, `table declaration`, or `comment`, found `{:?}`",
                                root_group.as_rule()
                            ))]),
                );

                return Err(Error::UnexpectedPair(root_group.as_span().into()));
            }
        }
    }

    Ok(Schema { items })
}

#[inline]
fn parse_enum<'i>(ctx: &mut Context, pair: Pair<'i, Rule>) -> Result<Enum<'i>, Error> {
    debug_assert!(
        pair.as_rule() == Rule::decl_enum,
        "The root pair must be a `decl_enum` to be able to parse a enum declaration"
    );

    let inner_span = pair.as_span();
    let mut inner: Pairs<'i, Rule> = pair.into_inner();

    let name = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::ident => pair.as_str(),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `ident`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected end of stream")
                    .with_labels(vec![Label::primary(
                        ctx.file_id,
                        span_range_end(inner_span),
                    )
                    .with_message("here")]),
            );

            return Err(Error::UnexpectedEOS);
        }
    };

    let not_exists = match inner.peek() {
        Some(peeked) if peeked.as_rule() == Rule::exists => {
            let _ = inner.next();

            true
        }
        _ => false,
    };

    let mut variants = Vec::with_capacity(inner.size_hint().0);

    for pair in inner {
        match pair.as_rule() {
            Rule::variant => variants.push(pair.as_str()),
            _ => {
                ctx.diags.push(
                    Diagnostic::error()
                        .with_message("Unexpected token")
                        .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                            .with_message(format!(
                                "expected `variant`, found `{:?}`",
                                pair.as_rule()
                            ))]),
                );

                return Err(Error::UnexpectedPair(pair.as_span().into()));
            }
        }
    }

    Ok(Enum {
        name,
        not_exists,
        variants,
    })
}

#[inline]
fn parse_table<'i>(ctx: &mut Context, pair: Pair<'i, Rule>) -> Result<Table<'i>, Error> {
    debug_assert!(
        pair.as_rule() == Rule::decl_table,
        "The root pair must be a `decl_table` to be able to parse a table declaration"
    );

    let inner_span = pair.as_span();
    let mut inner: Pairs<'i, Rule> = pair.into_inner();

    let name = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::ident => pair.as_str(),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `ident`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected end of stream")
                    .with_labels(vec![Label::primary(
                        ctx.file_id,
                        span_range_end(inner_span),
                    )
                    .with_message("here")]),
            );

            return Err(Error::UnexpectedEOS);
        }
    };

    let not_exists = match inner.peek() {
        Some(peeked) if peeked.as_rule() == Rule::exists => {
            let _ = inner.next();

            true
        }
        _ => false,
    };

    let mut columns = Vec::new();
    let mut primary_keys = Vec::new();
    let mut foreign_keys = Vec::new();
    let mut unique_keys = Vec::new();

    for pair in inner {
        match pair.as_rule() {
            Rule::column => {
                let (col, modifiers) = parse_column(ctx, pair)?;
                let mut default = ColumnDefault::default();

                for modifier in modifiers {
                    match modifier {
                        Modifier::Default { value } => default = ColumnDefault::Raw(value),
                        Modifier::DefaultDateTime => default = ColumnDefault::Now,
                        Modifier::DefaultNull => default = ColumnDefault::Null,
                        Modifier::PrimaryKey => primary_keys.push(col.name),
                        Modifier::Reference {
                            table,
                            column,
                            delete,
                            update,
                        } => foreign_keys.push(ForeignKey {
                            local: col.name,
                            table,
                            foreign: column,
                            delete: delete.clone(),
                            update: update.clone(),
                        }),
                        Modifier::Unique => unique_keys.push(col.name),
                    }
                }

                columns.push(Column {
                    name: col.name,
                    typ: col.typ,
                    null: col.null,
                    default,
                });
            }
            Rule::comment => continue,
            _ => {
                ctx.diags.push(
                    Diagnostic::error()
                        .with_message("Unexpected token")
                        .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                            .with_message(format!(
                                "expected `column` or `comment`, found `{:?}`",
                                pair.as_rule()
                            ))]),
                );

                return Err(Error::UnexpectedPair(pair.as_span().into()));
            }
        }
    }

    Ok(Table {
        name,
        not_exists,
        columns,
        primary_keys,
        foreign_keys,
        unique_keys,
    })
}

#[inline]
fn parse_column<'i>(
    ctx: &mut Context,
    pair: Pair<'i, Rule>,
) -> Result<(ColumnPartial<'i>, Vec<Modifier<'i>>), Error> {
    debug_assert!(
        pair.as_rule() == Rule::column,
        "The root pair must be a `column` to be able to parse a table column definition"
    );

    let inner_span = pair.as_span();
    let mut inner: Pairs<'i, Rule> = pair.into_inner();

    let name = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::ident => pair.as_str(),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `ident`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected end of stream")
                    .with_labels(vec![Label::primary(
                        ctx.file_id,
                        span_range_end(inner_span),
                    )
                    .with_message("here")]),
            );

            return Err(Error::UnexpectedEOS);
        }
    };

    let typ = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::column_type => Types::from_str(pair.as_str()),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `column type`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected end of stream")
                    .with_labels(vec![Label::primary(
                        ctx.file_id,
                        span_range_end(inner_span),
                    )
                    .with_message("here")]),
            );

            return Err(Error::UnexpectedEOS);
        }
    };

    let null = match inner.peek() {
        Some(peeked) if peeked.as_rule() == Rule::null => {
            let _ = inner.next();

            true
        }
        _ => false,
    };

    let modifiers = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::modifiers => parse_modifiers(ctx, pair)?,
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `modifiers`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => vec![],
    };

    Ok((ColumnPartial { name, typ, null }, modifiers))
}

#[inline]
fn parse_modifiers<'i>(
    ctx: &mut Context,
    pair: Pair<'i, Rule>,
) -> Result<Vec<Modifier<'i>>, Error> {
    debug_assert!(
        pair.as_rule() == Rule::modifiers,
        "The root pair must be a `modifiers` to be able to parse column modifiers"
    );

    let inner_span = pair.as_span();
    let inner: Pairs<'i, Rule> = pair.into_inner();

    let mut modifiers = Vec::new();

    for pair in inner {
        match pair.as_rule() {
            Rule::modifier_default => {
                let mut inner = pair.into_inner();

                let default = match inner.next() {
                    Some(pair) if pair.as_rule() == Rule::modifier_default_value => pair.as_str(),
                    Some(pair) => {
                        ctx.diags.push(
                            Diagnostic::error()
                                .with_message("Unexpected token")
                                .with_labels(vec![Label::primary(
                                    ctx.file_id,
                                    span_range_single(&pair),
                                )
                                .with_message(format!(
                                    "expected `modifier default value`, found `{:?}`",
                                    pair.as_rule()
                                ))]),
                        );

                        return Err(Error::UnexpectedPair(pair.as_span().into()));
                    }
                    None => {
                        ctx.diags.push(
                            Diagnostic::error()
                                .with_message("Unexpected end of stream")
                                .with_labels(vec![Label::primary(
                                    ctx.file_id,
                                    span_range_end(inner_span),
                                )
                                .with_message("here")]),
                        );

                        return Err(Error::UnexpectedEOS);
                    }
                };

                modifiers.push(match default {
                    "now()" => Modifier::DefaultDateTime,
                    "null" => Modifier::DefaultNull,
                    value => Modifier::Default { value },
                });
            }
            Rule::modifier_primary => modifiers.push(Modifier::PrimaryKey),
            Rule::modifier_ref => {
                let modifier = parse_modifier_ref(ctx, pair)?;
                modifiers.push(modifier);
            }
            Rule::modifier_unique => modifiers.push(Modifier::Unique),
            _ => {
                ctx.diags.push(
                    Diagnostic::error()
                        .with_message("Unexpected token")
                        .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!("expected `modifier default`, `modifier primary`, `modifier reference`, or `modifier unique`, found `{:?}`", pair.as_rule()))]),
                );

                return Err(Error::UnexpectedPair(pair.as_span().into()));
            }
        }
    }

    Ok(modifiers)
}

#[inline]
fn parse_modifier_ref<'i>(ctx: &mut Context, pair: Pair<'i, Rule>) -> Result<Modifier<'i>, Error> {
    debug_assert!(
        pair.as_rule() == Rule::modifier_ref,
        "The root pair must be a `modifier_ref` to be able to parse column ref modifier"
    );

    let inner_span = pair.as_span();
    let mut inner: Pairs<'i, Rule> = pair.into_inner();

    let table = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::ident => pair.as_str(),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `ident`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected end of stream")
                    .with_labels(vec![Label::primary(
                        ctx.file_id,
                        span_range_end(inner_span),
                    )
                    .with_message("here")]),
            );

            return Err(Error::UnexpectedEOS);
        }
    };

    let column = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::ident => pair.as_str(),
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `ident`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected end of stream")
                    .with_labels(vec![Label::primary(
                        ctx.file_id,
                        span_range_end(inner_span),
                    )
                    .with_message("here")]),
            );

            return Err(Error::UnexpectedEOS);
        }
    };

    let (delete, update) = match inner.next() {
        Some(pair) if pair.as_rule() == Rule::ref_action => parse_modifier_ref_action(ctx, pair)?,
        Some(pair) => {
            ctx.diags.push(
                Diagnostic::error()
                    .with_message("Unexpected token")
                    .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!(
                            "expected `modifier reference action(s)`, found `{:?}`",
                            pair.as_rule()
                        ))]),
            );

            return Err(Error::UnexpectedPair(pair.as_span().into()));
        }
        None => (Action::default(), Action::default()),
    };

    Ok(Modifier::Reference {
        table,
        column,
        delete,
        update,
    })
}

#[inline]
fn parse_modifier_ref_action<'i>(
    ctx: &mut Context,
    pair: Pair<'i, Rule>,
) -> Result<(Action, Action), Error> {
    debug_assert!(
        pair.as_rule() == Rule::ref_action,
        "The root pair must be a `ref_action` to be able to parse column ref modifier"
    );

    let inner_span = pair.as_span();
    let inner: Pairs<'i, Rule> = pair.into_inner();

    let mut delete = Action::default();
    let mut update = Action::default();

    for pair in inner {
        let (rule, action) = match pair.as_rule() {
            Rule::ref_action_delete => {
                let mut inner: Pairs<'i, Rule> = pair.into_inner();

                match inner.next() {
                    Some(pair) if pair.as_rule() == Rule::action => {
                        (Rule::ref_action_delete, pair.as_str())
                    }
                    Some(pair) => {
                        ctx.diags.push(
                            Diagnostic::error()
                                .with_message("Unexpected token")
                                .with_labels(vec![Label::primary(
                                    ctx.file_id,
                                    span_range_single(&pair),
                                )
                                .with_message(format!(
                                    "expected `action`, found `{:?}`",
                                    pair.as_rule()
                                ))]),
                        );

                        return Err(Error::UnexpectedPair(pair.as_span().into()));
                    }
                    None => {
                        ctx.diags.push(
                            Diagnostic::error()
                                .with_message("Unexpected end of stream")
                                .with_labels(vec![Label::primary(
                                    ctx.file_id,
                                    span_range_end(inner_span),
                                )
                                .with_message("here")]),
                        );

                        return Err(Error::UnexpectedEOS);
                    }
                }
            }
            Rule::ref_action_update => {
                let mut inner: Pairs<'i, Rule> = pair.into_inner();

                match inner.next() {
                    Some(pair) if pair.as_rule() == Rule::action => {
                        (Rule::ref_action_update, pair.as_str())
                    }
                    Some(pair) => {
                        ctx.diags.push(
                            Diagnostic::error()
                                .with_message("Unexpected token")
                                .with_labels(vec![Label::primary(
                                    ctx.file_id,
                                    span_range_single(&pair),
                                )
                                .with_message(format!(
                                    "expected `action`, found `{:?}`",
                                    pair.as_rule()
                                ))]),
                        );

                        return Err(Error::UnexpectedPair(pair.as_span().into()));
                    }
                    None => {
                        ctx.diags.push(
                            Diagnostic::error()
                                .with_message("Unexpected end of stream")
                                .with_labels(vec![Label::primary(
                                    ctx.file_id,
                                    span_range_end(inner_span),
                                )
                                .with_message("here")]),
                        );

                        return Err(Error::UnexpectedEOS);
                    }
                }
            }
            _ => {
                ctx.diags.push(
                    Diagnostic::error()
                        .with_message("Unexpected token")
                        .with_labels(vec![Label::primary(ctx.file_id, span_range_single(&pair))
                        .with_message(format!("expected `modifier reference action delete`, or `modifier reference action update`, found `{:?}`", pair.as_rule()))]),
                );

                return Err(Error::UnexpectedPair(pair.as_span().into()));
            }
        };

        match rule {
            Rule::ref_action_delete => delete = Action::try_from(action)?,
            Rule::ref_action_update => update = Action::try_from(action)?,
            _ => unreachable!(),
        }
    }

    Ok((delete, update))
}

#[cfg(test)]
mod tests {
    pub use {
        super::*,
        crate::models::{Column, Item, Table},
        codespan_reporting::{
            files::SimpleFiles,
            term::{
                self,
                termcolor::{Buffer, ColorChoice, StandardStream, WriteColor},
                Config,
            },
        },
    };

    fn assert_span(name: &str, input: &str, out: Schema) {
        let mut files = SimpleFiles::new();

        let file_id = files.add(name, input);

        let mut ctx = Context::new(file_id);

        match parse(&mut ctx, input) {
            Ok(schema) => assert_eq!(out, schema,),
            Err(err) => {
                let mut writer = Buffer::no_color();
                let config = Config::default();

                for diag in ctx.diags {
                    term::emit(&mut writer, &config, &files, &diag).unwrap();
                }

                panic!(
                    "{}{:?}",
                    String::from_utf8_lossy(writer.as_slice()).into_owned(),
                    err
                );
            }
        }
    }

    mod enums {
        use super::*;

        const ENUM: &str = "enum Rating {
            Explicit
            Mature
            Teen
            General
        }";

        #[test]
        fn simple() {
            assert_span(
                "tests::enums::simple",
                ENUM,
                Schema {
                    items: vec![Item::Enum(Enum {
                        name: "Rating",
                        not_exists: false,
                        variants: vec!["Explicit", "Mature", "Teen", "General"],
                    })],
                },
            );
        }
    }

    mod tables {
        use super::*;

        const TABLE: &str = "table Settings {
            key text [primary key]
            value text
            created dateTime [default: now()]
            updated dateTime [default: now()]
        }";

        const TABLE_NULL: &str = "table Settings {
            key text [primary key]
            value text!
            created dateTime [default: now()]
            updated dateTime [default: now()]
        }";

        const TABLE_REFERENCE: &str = "table Settings {
            key text [primary key]
            otherOne text [ref: Other.id (delete: cascade, update: cascade)]
            otherTwo text [ref: Other.id (delete: cascade)]
            otherThree text [ref: Other.id (update: cascade)]
            created dateTime [default: now()]
            updated dateTime [default: now()]
        }";

        #[inline]
        fn def_table(column: Column) -> Schema {
            Schema {
                items: vec![Item::Table(Table {
                    name: "Settings",
                    not_exists: false,
                    columns: vec![
                        Column {
                            name: "key",
                            typ: Types::Text,
                            null: false,
                            default: ColumnDefault::default(),
                        },
                        column,
                        Column {
                            name: "created",
                            typ: Types::DateTime,
                            null: false,
                            default: ColumnDefault::Now,
                        },
                        Column {
                            name: "updated",
                            typ: Types::DateTime,
                            null: false,
                            default: ColumnDefault::Now,
                        },
                    ],
                    primary_keys: vec!["key"],
                    foreign_keys: vec![],
                    unique_keys: vec![],
                })],
            }
        }

        #[test]
        fn simple() {
            assert_span(
                "tests::tables::simple",
                TABLE,
                def_table(Column {
                    name: "value",
                    typ: Types::Text,
                    null: false,
                    default: ColumnDefault::default(),
                }),
            );
        }

        #[test]
        fn simple_null() {
            assert_span(
                "tests::tables::simple_null",
                TABLE_NULL,
                def_table(Column {
                    name: "value",
                    typ: Types::Text,
                    null: true,
                    default: ColumnDefault::default(),
                }),
            );
        }

        #[test]
        fn reference() {
            assert_span(
                "tests::tables::reference",
                TABLE_REFERENCE,
                Schema {
                    items: vec![Item::Table(Table {
                        name: "Settings",
                        not_exists: false,
                        columns: vec![
                            Column {
                                name: "key",
                                typ: Types::Text,
                                null: false,
                                default: ColumnDefault::default(),
                            },
                            Column {
                                name: "otherOne",
                                typ: Types::Text,
                                null: false,
                                default: ColumnDefault::default(),
                            },
                            Column {
                                name: "otherTwo",
                                typ: Types::Text,
                                null: false,
                                default: ColumnDefault::default(),
                            },
                            Column {
                                name: "otherThree",
                                typ: Types::Text,
                                null: false,
                                default: ColumnDefault::default(),
                            },
                            Column {
                                name: "created",
                                typ: Types::DateTime,
                                null: false,
                                default: ColumnDefault::Now,
                            },
                            Column {
                                name: "updated",
                                typ: Types::DateTime,
                                null: false,
                                default: ColumnDefault::Now,
                            },
                        ],
                        primary_keys: vec!["key"],
                        foreign_keys: vec![
                            ForeignKey {
                                local: "otherOne",
                                table: "Other",
                                foreign: "id",
                                delete: Action::Cascade,
                                update: Action::Cascade,
                            },
                            ForeignKey {
                                local: "otherTwo",
                                table: "Other",
                                foreign: "id",
                                delete: Action::Cascade,
                                update: Action::default(),
                            },
                            ForeignKey {
                                local: "otherThree",
                                table: "Other",
                                foreign: "id",
                                delete: Action::default(),
                                update: Action::Cascade,
                            },
                        ],
                        unique_keys: vec![],
                    })],
                },
            );
        }
    }
}
