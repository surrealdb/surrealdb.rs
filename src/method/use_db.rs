use crate::method::Method;
use crate::param::Param;
use crate::Connection;
use crate::Result;
use crate::Router;
use futures::future::BoxFuture;
use std::future::IntoFuture;
use surrealdb::sql::Value;

#[derive(Debug)]
pub struct UseDb<'r, C: Connection> {
    pub(super) router: Result<&'r Router<C>>,
    pub(super) db: String,
}

impl<'r, Client> IntoFuture for UseDb<'r, Client>
where
    Client: Connection,
{
    type Output = Result<()>;
    type IntoFuture = BoxFuture<'r, Result<()>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut conn = Client::new(Method::Use);
            conn.execute(self.router?, Param::new(vec![Value::None, self.db.into()]))
                .await
        })
    }
}
