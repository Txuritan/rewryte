# Rewryte

Rewryte is a tool to transform database schema files into SQL files for MySQL, PostgreSQL, and SQLite, while exporting a models for Rust projects.

## Database Abstraction Language

Rewryte uses a custom schema language called DAL or Database Abstraction Language, while it is in early stages it can still do a lot, from null columns to defaults.

### Examples

```
/* The question mark means `IF NOT EXISTS` */
table Example? {
    /* In order, name, type (optional null), modifiers */
    Id number [primary key]
    Name text [unique]
    Created dateTime [default: now()]
    Updated dateTime [default: now()]
}
```

References/foreign keys:

```
table Chapter? {
    Id text [primary key]

    Name text
    Main text

    Created dateTime [default: now()]
    Updated dateTime [default: now()]
}

table Story? {
    Id text [primary key]

    Name text

    Created dateTime [default: now()]
    Updated dateTime [default: now()]
}

table StoryChapter? {
    StoryId text [primary key, ref: Story.Id]
    ChapterId text [primary key, ref: Chapter.Id]

    Place number

    Created dateTime [default: now()]
    Updated dateTime [default: now()]
}
```

```
/* The question mark means `IF NOT EXISTS` */
enum State? {
    Working
    Finished
}
```

## Code Generation

Rewryte can generate helper models for its supported database formats, along with row to type conversion generation.

Note, it is a good idea to have with in a separate module to avoid naming collisions.

`schema.dal`:
```
table Settings {
    Key text [primary key]
    Value text

    Created dateTime [default: now()]
    Updated dateTime [default: now()]
}
```

`lib.rs`:
```rust
rewryte::models!("./schema.dal", ["sqlite"]);
```

`lib.rs`:
```rust
use anyhow::Context;

struct Settings {
    key: String,
    value: String,
    created: chrono::DateTime<chrono::Utc>,
    updated: chrono::DateTime<chrono::Utc>,
}

impl rewryte::sqlite::FromRow for Settings {
    fn from_row(row: &rewryte::sqlite::Row<'_>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            key: row.get(0).context("Failed to get data for row index 0")?,
            value: row.get(1).context("Failed to get data for row index 1")?,
            created: row.get(2).context("Failed to get data for row index 2")?,
            updated: row.get(3).context("Failed to get data for row index 3")?,
        })
    }
}
```
