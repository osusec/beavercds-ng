use futures::{future::try_join_all, TryFuture};

/// Helper trait for `Iterator` to add futures::try_await_all() as chain method.
///
/// The same as wrapping it in `try_join_all(...).await`, but as a chained
/// method instead for cleaner readability.
#[allow(async_fn_in_trait)]
pub trait TryJoinAll: IntoIterator
where
    Self::Item: TryFuture,
{
    async fn try_join_all(
        self,
    ) -> Result<Vec<<Self::Item as TryFuture>::Ok>, <Self::Item as TryFuture>::Error>;
}

impl<I> TryJoinAll for I
where
    I: IntoIterator,
    I::Item: TryFuture,
{
    /// futures::try_join_all() as a iterator chain method
    async fn try_join_all(
        self,
    ) -> Result<Vec<<Self::Item as TryFuture>::Ok>, <Self::Item as TryFuture>::Error> {
        try_join_all(self).await
    }
}
