pub use rusqlite::*;

use std::marker::PhantomData;

#[macro_export]
macro_rules! sqlite_named_params {
        () => {
            &[]
        };
        ($($param_name:literal: $param_val:expr),+ $(,)?) => {
            &[$(($param_name, &$param_val as &dyn $crate::sqlite::ToSql)),+]
        };
    }

#[macro_export]
macro_rules! sqlite_params {
        () => {
            &[]
        };
        ($($param:expr),+ $(,)?) => {
            &[$(&$param as &dyn $crate::sqlite::ToSql),+] as &[&dyn $crate::sqlite::ToSql]
        };
    }

pub trait FromRow {
    fn from_row(row: &Row<'_>) -> anyhow::Result<Self>
    where
        Self: Sized;
}

/// An iterator over the mapped resulting rows of a query.
///
/// `F` is used to transform the _streaming_ iterator into a _standard_ iterator.
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct MappedRowsExt<'stmt, F> {
    rows: Rows<'stmt>,
    map: F,
}

impl<'stmt, T, F> MappedRowsExt<'stmt, F>
where
    F: FnMut(&Row<'_>) -> anyhow::Result<T>,
{
    pub(crate) fn new(rows: Rows<'stmt>, f: F) -> Self {
        Self { rows, map: f }
    }
}

impl<T, F> Iterator for MappedRowsExt<'_, F>
where
    F: FnMut(&Row<'_>) -> anyhow::Result<T>,
{
    type Item = anyhow::Result<T>;

    fn next(&mut self) -> Option<anyhow::Result<T>> {
        let map = &mut self.map;

        self.rows
            .next()
            .map_err(anyhow::Error::from)
            .transpose()
            .map(|row_result| {
                row_result
                    .and_then(|row| (map)(&row))
                    .map_err(anyhow::Error::from)
            })
    }
}
pub struct TypeMappedRowsExt<'stmt, T> {
    rows: Rows<'stmt>,
    typ: PhantomData<T>,
}

impl<'stmt, T> TypeMappedRowsExt<'stmt, T>
where
    T: FromRow,
{
    pub(crate) fn new(rows: Rows<'stmt>) -> Self {
        Self {
            rows,
            typ: PhantomData::default(),
        }
    }
}

impl<T> Iterator for TypeMappedRowsExt<'_, T>
where
    T: FromRow,
{
    type Item = anyhow::Result<T>;

    fn next(&mut self) -> Option<anyhow::Result<T>> {
        self.rows
            .next()
            .map_err(anyhow::Error::from)
            .transpose()
            .map(|row_result| {
                row_result
                    .and_then(|row| T::from_row(&row))
                    .map_err(anyhow::Error::from)
            })
    }
}

pub trait ConnectionExt {
    fn query_one<T, P, F>(&self, sql: &str, params: P, f: F) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>;

    fn query_one_opt<T, P, F>(&self, sql: &str, params: P, f: F) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>;

    fn type_query_one<T, P>(&self, sql: &str, params: P) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow;

    fn type_query_one_opt<T, P>(&self, sql: &str, params: P) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow;
}

impl ConnectionExt for rusqlite::Connection {
    fn query_one<T, P, F>(&self, sql: &str, params: P, f: F) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>,
    {
        let mut stmt = self.prepare(sql)?;

        let row = stmt.query_one(params, f)?;

        Ok(row)
    }

    fn query_one_opt<T, P, F>(&self, sql: &str, params: P, f: F) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>,
    {
        match self.query_one(sql, params, f) {
            Ok(res) => Ok(Some(res)),
            Err(err) => match err.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                _ => Err(err),
            },
        }
    }

    fn type_query_one<T, P>(&self, sql: &str, params: P) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow,
    {
        let mut stmt = self.prepare(sql)?;

        let row = stmt.type_query_one(params)?;

        Ok(row)
    }

    fn type_query_one_opt<T, P>(&self, sql: &str, params: P) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow,
    {
        match self.type_query_one(sql, params) {
            Ok(res) => Ok(Some(res)),
            Err(err) => match err.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                _ => Err(err),
            },
        }
    }
}

pub trait StatementExt {
    fn query<T, P, F>(&mut self, params: P, f: F) -> anyhow::Result<MappedRowsExt<'_, F>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnMut(&Row<'_>) -> anyhow::Result<T>;

    fn query_opt<T, P, F>(
        &mut self,
        params: P,
        f: F,
    ) -> anyhow::Result<Option<MappedRowsExt<'_, F>>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnMut(&Row<'_>) -> anyhow::Result<T>;

    fn type_query<T, P>(&mut self, params: P) -> anyhow::Result<TypeMappedRowsExt<'_, T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow;

    fn type_query_opt<T, P>(
        &mut self,
        params: P,
    ) -> anyhow::Result<Option<TypeMappedRowsExt<'_, T>>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow;

    fn query_one<T, P, F>(&mut self, params: P, f: F) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>;

    fn type_query_one<T, P>(&mut self, params: P) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow;

    fn query_one_opt<T, P, F>(&mut self, params: P, f: F) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>;

    fn type_query_one_opt<T, P>(&mut self, params: P) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow;
}

impl StatementExt for rusqlite::Statement<'_> {
    fn query<T, P, F>(&mut self, params: P, f: F) -> anyhow::Result<MappedRowsExt<'_, F>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnMut(&Row<'_>) -> anyhow::Result<T>,
    {
        let rows = self.query(params)?;

        Ok(MappedRowsExt::new(rows, f))
    }

    fn query_opt<T, P, F>(
        &mut self,
        params: P,
        f: F,
    ) -> anyhow::Result<Option<MappedRowsExt<'_, F>>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnMut(&Row<'_>) -> anyhow::Result<T>,
    {
        let rows = match self.query(params).map_err(anyhow::Error::from) {
            Ok(rows) => rows,
            Err(err) => match err.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                _ => return Err(err),
            },
        };

        Ok(Some(MappedRowsExt::new(rows, f)))
    }

    fn type_query<T, P>(&mut self, params: P) -> anyhow::Result<TypeMappedRowsExt<'_, T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow,
    {
        let rows = self.query(params)?;

        Ok(TypeMappedRowsExt::new(rows))
    }

    fn type_query_opt<T, P>(
        &mut self,
        params: P,
    ) -> anyhow::Result<Option<TypeMappedRowsExt<'_, T>>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow,
    {
        let rows = match self.query(params).map_err(anyhow::Error::from) {
            Ok(rows) => rows,
            Err(err) => match err.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                _ => return Err(err),
            },
        };

        Ok(Some(TypeMappedRowsExt::new(rows)))
    }

    fn query_one<T, P, F>(&mut self, params: P, f: F) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>,
    {
        let mut rows = self.query(params)?;

        match rows.next()? {
            Some(row) => Ok(f(&row)?),
            None => Err(rusqlite::Error::QueryReturnedNoRows.into()),
        }
    }

    fn query_one_opt<T, P, F>(&mut self, params: P, f: F) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>,
    {
        let mut rows = match self.query(params).map_err(anyhow::Error::from) {
            Ok(rows) => rows,
            Err(err) => match err.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                _ => return Err(err),
            },
        };

        let res: Option<T> = match rows.next()? {
            Some(row) => Some(f(&row)?),
            None => None,
        };

        Ok(res)
    }

    fn type_query_one<T, P>(&mut self, params: P) -> anyhow::Result<T>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow,
    {
        let mut rows = self.query(params)?;

        match rows.next()? {
            Some(row) => Ok(T::from_row(&row)?),
            None => Err(rusqlite::Error::QueryReturnedNoRows.into()),
        }
    }

    fn type_query_one_opt<T, P>(&mut self, params: P) -> anyhow::Result<Option<T>>
    where
        P: IntoIterator,
        P::Item: ToSql,
        T: FromRow,
    {
        let mut rows = match self.query(params).map_err(anyhow::Error::from) {
            Ok(rows) => rows,
            Err(err) => match err.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                _ => return Err(err),
            },
        };

        let res: Option<T> = match rows.next()? {
            Some(row) => Some(T::from_row(&row)?),
            None => None,
        };

        Ok(res)
    }
}
