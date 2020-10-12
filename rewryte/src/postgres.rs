pub use tokio_postgres::*;

use {
    anyhow::Context as _,
    futures::{Stream, TryStreamExt},
    std::{
        collections::HashMap,
        marker::{PhantomData, PhantomPinned},
        net::IpAddr,
        pin::Pin,
        task::{Context, Poll},
        time::SystemTime,
    },
    tokio_postgres::types::ToSql,
};

#[macro_export]
macro_rules! postgres_params {
        () => {
            &[] as &[&(dyn $crate::postgres::types::ToSql + Sync)]
        };
        ($( $param:expr ),+ $(,)?) => {
            &[$(&$param as &(dyn $crate::postgres::types::ToSql + Sync)),+] as &[&(dyn $crate::postgres::types::ToSql + Sync)]
        };
    }

fn slice_iter<'a>(
    s: &'a [&'a (dyn ToSql + Sync)],
) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
    s.iter().map(|s| *s as _)
}

pub trait FromRow {
    fn from_row(row: Row) -> anyhow::Result<Self>
    where
        Self: Sized;
}

macro_rules! impl_from_row {
    ($( $from:ty, )*) => {
        $(
            impl FromRow for $from {
                fn from_row(row: Row) -> anyhow::Result<Self>
                where
                    Self: Sized,
                {
                    row.try_get(0)
                        .context(concat!("Failed to get data for row index 0: `", stringify!($from), "`"))
                }
            }
        )*
    };
}

impl_from_row![
    bool,
    i8, i16, i32, u32, i64,
    f32, f64,
    String, Vec<u8>,
    HashMap<String, Option<String>>,
    SystemTime,
    IpAddr,
];

#[cfg(feature = "with-chrono")]
impl_from_row![
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
];

#[cfg(feature = "with-chrono")]
impl FromRow for chrono::DateTime<chrono::Local> {
    fn from_row(row: Row) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        row.try_get(0)
            .context(concat!("Failed to get data for row index 0: `DateTime`"))
    }
}

#[cfg(feature = "with-serde-json")]
impl_from_row![
    serde_json::Value,
];

#[cfg(feature = "with-uuid")]
impl_from_row![
    uuid::Uuid,
];

#[cfg(feature = "with-chrono")]
impl FromRow for chrono::DateTime<chrono::Utc> {
    fn from_row(row: Row) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        row.try_get(0)
            .context(concat!("Failed to get data for row index 0: `DateTime`"))
    }
}

#[async_trait::async_trait]
pub trait ClientExt {
    async fn type_query<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Vec<T>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync;

    async fn type_query_opt<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<Vec<T>>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync;

    async fn type_query_one<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<T>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync;

    async fn type_query_one_opt<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<T>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync;

    async fn type_query_raw<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<TypedRowStreamExt<T>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow;
}

#[async_trait::async_trait]
impl ClientExt for Client {
    async fn type_query<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Vec<T>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync,
    {
        self.type_query_raw::<T, S>(statement, params)
            .await?
            .try_collect()
            .await
    }

    async fn type_query_opt<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<Vec<T>>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync,
    {
        let stream = self.type_query_raw::<T, S>(statement, params).await?;

        futures::pin_mut!(stream);

        let mut buff = None;

        while let Some(item) = stream.try_next().await? {
            buff.get_or_insert_with(|| Vec::with_capacity(stream.size_hint().0))
                .push(item);
        }

        Ok(buff)
    }

    async fn type_query_one<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<T>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync,
    {
        let stream = self.type_query_raw::<T, S>(statement, params).await?;

        futures::pin_mut!(stream);

        let row = match stream.try_next().await? {
            Some(row) => row,
            None => anyhow::bail!("query returned an unexpected number of rows"),
        };

        if stream.try_next().await?.is_some() {
            anyhow::bail!("query returned an unexpected number of rows");
        }

        Ok(row)
    }

    async fn type_query_one_opt<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<Option<T>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow + Send + Sync,
    {
        let stream = self.type_query_raw::<T, S>(statement, params).await?;

        futures::pin_mut!(stream);

        let row = match stream.try_next().await? {
            Some(row) => row,
            None => return Ok(None),
        };

        if stream.try_next().await?.is_some() {
            anyhow::bail!("query returned an unexpected number of rows");
        }

        Ok(Some(row))
    }

    async fn type_query_raw<T, S>(
        &self,
        statement: &S,
        params: &[&(dyn ToSql + Sync)],
    ) -> anyhow::Result<TypedRowStreamExt<T>>
    where
        S: ?Sized + ToStatement + Send + Sync,
        T: FromRow,
    {
        let stream = self.query_raw(statement, slice_iter(params)).await?;

        Ok(TypedRowStreamExt {
            stream,
            _p: PhantomPinned,
            _t: PhantomData,
        })
    }
}

pin_project_lite::pin_project! {
    /// A stream of the mapped resulting table rows.
    pub struct TypedRowStreamExt<T>
    where
        T: FromRow,
    {
        #[pin]
        stream: RowStream,
        #[pin]
        _p: PhantomPinned,
        _t: PhantomData<T>,
    }
}

impl<T> Stream for TypedRowStreamExt<T>
where
    T: FromRow,
{
    type Item = Result<T, anyhow::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let polled: Option<Row> = futures::ready!(this.stream.poll_next(cx)?);

        match polled {
            Some(row) => Poll::Ready(Some(T::from_row(row))),
            None => Poll::Ready(None),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
