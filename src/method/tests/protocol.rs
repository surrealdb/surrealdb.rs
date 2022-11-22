use super::server;
use crate::method::query_response::QueryResponse;
use crate::param::from_value;
use crate::param::DbResponse;
use crate::param::Param;
use crate::param::ServerAddrs;
use crate::param::ToServerAddrs;
use crate::Connection;
use crate::Method;
use crate::Result;
use crate::Route;
use crate::Router;
use crate::Surreal;
use flume::Receiver;
use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
#[cfg(feature = "ws")]
use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use url::Url;

#[derive(Debug)]
pub struct Test;

impl ToServerAddrs<Test> for () {
	type Client = Client;

	fn to_server_addrs(self) -> Result<ServerAddrs> {
		Ok(ServerAddrs {
			endpoint: Url::parse("test://localhost:8000")?,
			strict: false,
			#[cfg(any(feature = "native-tls", feature = "rustls"))]
			tls_config: None,
		})
	}
}

#[derive(Debug, Clone)]
pub struct Client {
	method: Method,
}

impl Connection for Client {
	type Request = (Method, Param);
	type Response = Result<DbResponse>;

	fn new(method: Method) -> Self {
		Self {
			method,
		}
	}

	fn connect(
		_address: ServerAddrs,
		capacity: usize,
	) -> Pin<Box<dyn Future<Output = Result<Surreal<Self>>> + Send + Sync + 'static>> {
		Box::pin(async move {
			let (route_tx, route_rx) = flume::bounded(capacity);
			let router = Router {
				conn: PhantomData,
				sender: route_tx,
				#[cfg(feature = "ws")]
				last_id: AtomicI64::new(0),
			};
			server::mock(route_rx);
			Ok(Surreal {
				router: OnceCell::with_value(Arc::new(router)),
			})
		})
	}

	fn send<'r>(
		&'r mut self,
		router: &'r Router<Self>,
		param: Param,
	) -> Pin<Box<dyn Future<Output = Result<Receiver<Self::Response>>> + Send + Sync + 'r>> {
		Box::pin(async move {
			let (sender, receiver) = flume::bounded(1);
			let route = Route {
				request: (self.method, param),
				response: sender,
			};
			router
				.sender
				.send_async(Some(route))
				.await
				.as_ref()
				.map_err(ToString::to_string)
				.unwrap();
			Ok(receiver)
		})
	}

	fn recv<R>(
		&mut self,
		rx: Receiver<Self::Response>,
	) -> Pin<Box<dyn Future<Output = Result<R>> + Send + Sync + '_>>
	where
		R: DeserializeOwned,
	{
		Box::pin(async move {
			let result = rx.into_recv_async().await.unwrap();
			match result.unwrap() {
				DbResponse::Other(value) => from_value(&value),
				DbResponse::Query(..) => unreachable!(),
			}
		})
	}

	fn recv_query(
		&mut self,
		rx: Receiver<Self::Response>,
	) -> Pin<Box<dyn Future<Output = Result<QueryResponse>> + Send + Sync + '_>> {
		Box::pin(async move {
			let result = rx.into_recv_async().await.unwrap();
			match result.unwrap() {
				DbResponse::Query(results) => Ok(results),
				DbResponse::Other(..) => unreachable!(),
			}
		})
	}
}
