use crate::method::Method;
use crate::param;
use crate::param::from_json;
use crate::param::Param;
use crate::Connection;
use crate::Response;
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

/// A query future
#[derive(Debug)]
pub struct Query<'r, C: Connection> {
    pub(in super::super) router: Result<&'r Router<C>>,
    pub(in super::super) query: Vec<Result<Vec<Statement>>>,
    pub(in super::super) bindings: BTreeMap<String, Value>,
}

impl<'r, Client> IntoFuture for Query<'r, Client>
where
    Client: Connection,
{
    type Output = Result<Response>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut statements = Vec::with_capacity(self.query.len());
            for query in self.query {
                statements.extend(query?);
            }
            let mut param = vec![sql::Query(Statements(statements)).to_string().into()];
            if !self.bindings.is_empty() {
                param.push(self.bindings.into());
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

    /// Binds a parameter to a query
    pub fn bind<D>(mut self, key: impl Into<String>, value: D) -> Self
    where
        D: Serialize,
    {
        self.bindings.insert(key.into(), from_json(json!(value)));
        self
    }
}
