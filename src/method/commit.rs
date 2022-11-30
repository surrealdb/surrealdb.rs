use crate::Connection;
use crate::Result;
use crate::Surreal;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use surrealdb::sql::statements::CommitStatement;

/// A transaction commit future
#[derive(Debug)]
pub struct Commit<C: Connection> {
	pub(crate) client: Surreal<C>,
}

impl<C> IntoFuture for Commit<C>
where
	C: Connection,
{
	type Output = Result<Surreal<C>>;
	type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'static>>;

	fn into_future(self) -> Self::IntoFuture {
		Box::pin(async move {
			self.client.query(CommitStatement).await?;
			Ok(self.client)
		})
	}
}
