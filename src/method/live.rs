use crate::method::Method;
use crate::param::Param;
use crate::Connection;
use crate::Result;
use crate::Router;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use surrealdb::sql::Uuid;

/// A live query future
#[derive(Debug)]
pub struct Live<'r, C: Connection> {
    pub(super) router: Result<&'r Router<C>>,
    pub(super) table_name: String,
}

impl<'r, Client> IntoFuture for Live<'r, Client>
where
    Client: Connection,
{
    type Output = Result<Uuid>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut conn = Client::new(Method::Live);
            conn.execute(self.router?, Param::new(vec![self.table_name.into()]))
                .await
        })
    }
}
