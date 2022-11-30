/// A module for the various wrapping types for the responses and results
/// returned by the database.
pub mod response;

use crate::method::Method;
use crate::param;
use crate::param::from_json;
use crate::param::Param;
use crate::Connection;
use crate::ErrorKind;
use crate::Result;
use crate::Router;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::future::Future;
use std::future::IntoFuture;
use std::mem;
use std::pin::Pin;
use surrealdb::sql;
use surrealdb::sql::Array;
use surrealdb::sql::Object;
use surrealdb::sql::Statement;
use surrealdb::sql::Statements;
use surrealdb::sql::Strand;
use surrealdb::sql::Value;

use response::QueryResponse;

/// A query future
#[derive(Debug)]
pub struct Query<'r, C: Connection> {
	pub(super) router: Result<&'r Router<C>>,
	pub(super) query: Vec<Result<Vec<Statement>>>,
	pub(super) bindings: Result<BTreeMap<String, Value>>,
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
			let mut param = vec![sql::Query(Statements(statements)).to_string().into()];
			let bindings = self.bindings?;
			if !bindings.is_empty() {
				param.push(bindings.into());
			}
			let mut conn = Client::new(Method::Query);
			conn.execute_query(self.router?, Param::new(param)).await
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

	/// Binds a parameter or parameters to a query
	///
	/// # Examples
	///
	/// Binding a key/value tuple
	///
	/// ```no_run
	/// # use surrealdb_rs::{Result, Surreal};
	/// # use surrealdb_rs::net::WsClient;
	/// # #[tokio::main]
	/// # async fn main() -> Result<()> {
	/// # let db = Surreal::<WsClient>::new();
	/// let response = db.query("CREATE user SET name = $name")
	///     .bind(("name", "John Doe"))
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// Binding an object
	///
	/// ```no_run
	/// # use serde::Serialize;
	/// # use surrealdb_rs::{Result, Surreal};
	/// # use surrealdb_rs::net::WsClient;
	///
	/// #[derive(Serialize)]
	/// struct User<'a> {
	///     name: &'a str,
	/// }
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<()> {
	/// # let db = Surreal::<WsClient>::new();
	/// let response = db.query("CREATE user SET name = $name")
	///     .bind(User {
	///         name: "John Doe",
	///     })
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn bind(mut self, bindings: impl Serialize) -> Self {
		if let Ok(current) = &mut self.bindings {
			let mut bindings = from_json(json!(bindings));
			if let Value::Array(Array(array)) = &mut bindings {
				if let [Value::Strand(Strand(key)), value] = &mut array[..] {
					let mut map = BTreeMap::new();
					map.insert(mem::take(key), mem::take(value));
					bindings = map.into();
				}
			}
			match &mut bindings {
				Value::Object(Object(map)) => current.append(map),
				_ => {
					self.bindings = Err(ErrorKind::InvalidBindings.with_context(&bindings));
				}
			}
		}
		self
	}
}
