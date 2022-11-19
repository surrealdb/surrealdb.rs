use crate::method::Method;
use crate::param::Param;
use crate::Connection;
use crate::Result;
use crate::Router;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use surrealdb::sql::Value;

/// A set future
#[derive(Debug)]
pub struct Set<'r, C: Connection> {
    pub(super) router: Result<&'r Router<C>>,
    pub(super) key: String,
    pub(super) value: Result<Value>,
}

impl<'r, Client> IntoFuture for Set<'r, Client>
where
    Client: Connection,
{
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut conn = Client::new(Method::Set);
            conn.execute(self.router?, Param::new(vec![self.key.into(), self.value?]))
                .await
        })
    }
}
