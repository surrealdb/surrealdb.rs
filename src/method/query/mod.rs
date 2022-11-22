/// A module for the various wrapping types for the responses and results
/// returned by the database.
pub mod response;

use crate::method::Method;
use crate::param;
use crate::param::from_json;
use crate::param::Param;
use crate::Connection;
use crate::Result;
use crate::Router;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use surrealdb::sql;
use surrealdb::sql::Statement;
use surrealdb::sql::Statements;
use surrealdb::sql::Value;

use response::QueryResponse;

/// A query future
#[derive(Debug)]
pub struct Query<'r, C: Connection> {
	pub(super) router: Result<&'r Router<C>>,
	pub(super) query: Vec<Result<Vec<Statement>>>,
	pub(super) bindings: BTreeMap<String, Value>,
}

impl<'r, Client> IntoFuture for Query<'r, Client>
where
	Client: Connection,
{
	type Output = Result<QueryResponse>;
	type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

	fn into_future(self) -> Self::IntoFuture {
		Box::pin(async move {
			let mut statements = Vec::with_capacity(self.query.len());
			for query in self.query {
				statements.extend(query?);
			}
			let query = sql::Query(Statements(statements));
			let param = Param::query(query, self.bindings);
			let mut conn = Client::new(Method::Query);
			conn.execute_query(self.router?, param).await
		})
	}
}

impl<'r, C> Query<'r, C>
where
	C: Connection,
{
	/// Chains a query onto an existing query
	pub fn query(mut self, query: impl param::Query) -> Self {
		self.query.push(query.try_into_query());
		self
	}

	/// Binds a parameter to a query
	pub fn bind<D>(mut self, key: impl Into<String>, value: D) -> Self
	where
		D: Serialize,
	{
		self.bindings.insert(key.into(), from_json(json!(value)));
		self
	}
}
