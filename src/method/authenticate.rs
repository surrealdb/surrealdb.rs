use crate::method::Method;
use crate::param::Jwt;
use crate::param::Param;
use crate::Connection;
use crate::Result;
use crate::Router;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;

/// An authentication future
#[derive(Debug)]
pub struct Authenticate<'r, C: Connection> {
	pub(super) router: Result<&'r Router<C>>,
	pub(super) token: Jwt,
}

impl<'r, Client> IntoFuture for Authenticate<'r, Client>
where
	Client: Connection,
{
	type Output = Result<()>;
	type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

	fn into_future(self) -> Self::IntoFuture {
		Box::pin(async move {
			let mut conn = Client::new(Method::Authenticate);
			conn.execute(self.router?, Param::new(vec![self.token.into()])).await
		})
	}
}
