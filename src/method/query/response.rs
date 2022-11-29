use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::Index;
use std::slice::SliceIndex;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::param::from_serializable;
use crate::Result;
use crate::Value;

/// A wrapper type around the list of results for the queries that were returned
/// by the database.
///
/// Provides utility functions to access the result of one specific query, or if
/// needed, all queries at once.
///
#[derive(Debug, Clone)]
pub struct QueryResponse(Vec<QueryResult>);

impl QueryResponse {
    /// Constructs an empty [`QueryResponse`]
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Unwrap into the inner list of query results
    pub fn into_inner(self) -> Vec<QueryResult> {
        self.0
    }

    /// Returns a reference the result for the `n`-th query from the response. If
    /// no result is found at this index then [None] is returned.
    pub fn get_query(&self, n: usize) -> Option<&QueryResult> {
        self.0.get(n)
    }

    /// Returns the deserialized [`<T>`] from the inner [Value]s over the given
    /// range or index for the query at `query_index`.
    ///
    /// - If no query is found at `query_index` then [`None`] is returned.
    /// - if `index_or_range` is an index of type [usize] then a single [`Option<T>`]
    /// is returned.
    /// - if `index_or_range` is a range then a [`Option<Vec<T>>`] is returned if
    /// and only if the full range is found inside the inner list.
    ///
    /// # Examples
    /// ```no_run
    /// # #[derive(Debug, serde::Deserialize)]
    /// # #[allow(dead_code)]
    /// # struct User {
    /// #   id: String,
    /// #   balance: String
    /// # }
    /// #
    /// # use surrealdb_rs::{Result, Surreal};
    /// # use surrealdb_rs::net::WsClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let client = Surreal::<WsClient>::new();
    /// # let token = String::new();
    /// let response = client.query("select * from user").await?;
    ///
    /// // get the first item from the first query
    /// let user: Option<User> = response.get(0, 0)?;
    /// tracing::info!("{user:?}");
    ///
    /// // get all items from the first query
    /// let users: Vec<User> = response.get(0, ..)?.unwrap_or_default();
    /// tracing::info!("{users:?}");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T, I>(&self, query_index: usize, index_or_range: I) -> Result<Option<T>>
    where
        T: DeserializeOwned,
        I: SliceIndex<[Value]>,
        <I as SliceIndex<[surrealdb::sql::Value]>>::Output: Serialize,
    {
        self.get_query(query_index)
            .and_then(|query_result| query_result.get(index_or_range).transpose())
            .transpose()
    }
}

impl Into<QueryResponse> for Vec<QueryResult> {
    fn into(self) -> QueryResponse {
        QueryResponse(self)
    }
}

impl FromIterator<Result<Vec<Value>>> for QueryResponse {
    fn from_iter<T: IntoIterator<Item = Result<Vec<Value>>>>(iter: T) -> Self {
        let mut query_results = Vec::new();

        for result in iter {
            query_results.push(result.into());
        }

        query_results.into()
    }
}

impl Deref for QueryResponse {
    type Target = [QueryResult];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl DerefMut for QueryResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0[..]
    }
}

impl IntoIterator for QueryResponse {
    type Item = QueryResult;
    type IntoIter = <Vec<QueryResult> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Index<usize> for QueryResponse {
    type Output = QueryResult;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl AsRef<Vec<QueryResult>> for QueryResponse {
    fn as_ref(&self) -> &Vec<QueryResult> {
        &self.0
    }
}

/// A wrapper type around the result of a single query.
///
/// Provides utility functions to deserialize items (of type [Value]) into any
/// [`<T>`] that implements [DeserializeOwned].
#[derive(Debug, Clone)]
pub struct QueryResult(Result<Vec<Value>>);

impl QueryResult {
    /// Returns the deserialized [`<T>`] from the inner [Value]s over the given
    /// range or index.
    ///
    /// - if `index_or_range` is an index of type [usize] then a single [`Option<T>`]
    /// is returned.
    /// - if `index_or_range` is a range then a [`Option<Vec<T>>`] is returned if
    /// and only if the full range is found inside the inner list.
    ///
    /// # Examples
    /// ```no_run
    /// # #[derive(Debug, serde::Deserialize)]
    /// # #[allow(dead_code)]
    /// # struct Account {
    /// #   id: String,
    /// #   balance: String
    /// # }
    /// #
    /// # use surrealdb_rs::{Result, Surreal};
    /// # use surrealdb_rs::net::WsClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let client = Surreal::<WsClient>::new();
    /// # let token = String::new();
    /// let response = client.query("select * from account where balance = '$100'").await?;
    ///
    /// if let Some(first_query_result) = response.get_query(0) {
    ///   // print the first account:
    ///   let account: Option<Account> = first_query_result.get(0)?;
    ///   dbg!(account);
    ///
    ///   // print the first two accounts, if at least two accounts are in the response:
    ///   let accounts: Option<Vec<Account>> = first_query_result.get(0..2)?;
    ///   dbg!(accounts);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T, I>(&self, index_or_range: I) -> Result<Option<T>>
    where
        T: DeserializeOwned,
        I: SliceIndex<[Value]>,
        <I as SliceIndex<[surrealdb::sql::Value]>>::Output: Serialize,
    {
        let values: &Vec<Value> = self.0.as_ref().map_err(|error| error.clone())?.as_ref();
        let some_slice = values.get::<I>(index_or_range);
        let items = match some_slice {
            Some(slice) => Some(from_serializable(slice)?),
            None => None,
        };

        Ok(items)
    }

    /// Returns the deserialized [`Vec<T>`] from the inner [Value]s, if no values
    /// are found then an empty [`Vec`] is returned instead.
    pub fn all<T>(&self) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        Ok(self.get(..)?.unwrap_or_default())
    }

    /// Returns a reference the inner [`Result`](crate::Result) with the raw
    /// unparsed [Value]s
    pub fn inner(&self) -> &Result<Vec<Value>> {
        &self.0
    }
}

impl Into<QueryResult> for Result<Vec<Value>> {
    fn into(self) -> QueryResult {
        QueryResult(self)
    }
}
