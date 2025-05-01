use anyhow::{Context, Result};
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

//
// Minijinja strict rendering with error
//

/// Similar to minijinja.render!(), but return Error if any undefined values.
pub fn render_strict(template: &str, context: minijinja::Value) -> Result<String> {
    let mut strict_env = minijinja::Environment::new();
    // error on any undefined template variables
    strict_env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);

    let r = strict_env
        .render_str(template, context)
        .context(format!("could not render template {:?}", template))?;
    Ok(r)
}
