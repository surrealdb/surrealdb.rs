#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, deny(warnings))]

//! This SurrealDB library enables simple and advanced querying of a remote or embedded database from
//! server-side or client-side code. All connections to SurrealDB are made over WebSockets by default (HTTP
//! and embedded databases are also supported), and automatically reconnect when the connection is terminated.
//!
//! # Examples
//!
//! ```no_run
//! use serde::{Serialize, Deserialize};
//! use serde_json::json;
//! use std::borrow::Cow;
//! use surrealdb_rs::{Result, Surreal};
//! use surrealdb_rs::param::Root;
//! use surrealdb_rs::protocol::Ws;
//! use ulid::Ulid;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Name {
//!     first: Cow<'static, str>,
//!     last: Cow<'static, str>,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Person {
//!     title: Cow<'static, str>,
//!     name: Name,
//!     marketing: bool,
//!     identifier: Ulid,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let db = Surreal::connect::<Ws>("127.0.0.1:8000").await?;
//!
//!     // Signin as a namespace, database, or root user
//!     db.signin(Root {
//!         username: "root",
//!         password: "root",
//!     }).await?;
//!
//!     // Select a specific namespace / database
//!     db.use_ns("test").use_db("test").await?;
//!
//!     // Create a new person with a random ID
//!     let created: Person = db.create("person")
//!         .content(Person {
//!             title: "Founder & CEO".into(),
//!             name: Name {
//!                 first: "Tobie".into(),
//!                 last: "Morgan Hitchcock".into(),
//!             },
//!             marketing: true,
//!             identifier: Ulid::new(),
//!         })
//!         .await?;
//!
//!     // Create a new person with a specific ID
//!     let created: Person = db.create(("person", "jaime"))
//!         .content(Person {
//!             title: "Founder & COO".into(),
//!             name: Name {
//!                 first: "Jaime".into(),
//!                 last: "Morgan Hitchcock".into(),
//!             },
//!             marketing: false,
//!             identifier: Ulid::new(),
//!         })
//!         .await?;
//!
//!     // Update a person record with a specific ID
//!     let updated: Person = db.update(("person", "jaime"))
//!         .merge(json!({"marketing": true}))
//!         .await?;
//!
//!     // Select all people records
//!     let people: Vec<Person> = db.select("person").await?;
//!
//!     // Perform a custom advanced query
//!     let groups = db
//!         .query("SELECT marketing, count() FROM type::table($tb) GROUP BY marketing")
//!         .bind("tb", "person")
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

mod err;

pub mod method;

#[cfg(any(
	feature = "mem",
	feature = "tikv",
	feature = "rocksdb",
	feature = "fdb",
	feature = "indxdb",
))]
#[cfg_attr(
	docsrs,
	doc(cfg(any(
		feature = "mem",
		feature = "tikv",
		feature = "rocksdb",
		feature = "fdb",
		feature = "indxdb",
	)))
)]
pub mod embedded;
#[cfg(any(feature = "http", feature = "ws"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http", feature = "ws"))))]
pub mod net;
pub mod param;
#[cfg(any(feature = "http", feature = "ws"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "http", feature = "ws"))))]
pub mod protocol;
#[cfg(any(
	feature = "mem",
	feature = "tikv",
	feature = "rocksdb",
	feature = "fdb",
	feature = "indxdb",
))]
#[cfg_attr(
	docsrs,
	doc(cfg(any(
		feature = "mem",
		feature = "tikv",
		feature = "rocksdb",
		feature = "fdb",
		feature = "indxdb",
	)))
)]
pub mod storage;

pub use err::Error;
pub use err::ErrorKind;
use method::query_response::QueryResponse;

use crate::param::ServerAddrs;
use crate::param::ToServerAddrs;
use flume::Receiver;
use flume::Sender;
use method::Method;
use once_cell::sync::OnceCell;
use semver::BuildMetadata;
use semver::VersionReq;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::future::Future;
use std::future::IntoFuture;
use std::marker::PhantomData;
use std::mem;
use std::pin::Pin;
#[cfg(feature = "ws")]
use std::sync::atomic::AtomicI64;
#[cfg(feature = "ws")]
use std::sync::atomic::Ordering;
use std::sync::Arc;
use surrealdb::sql::statements::CreateStatement;
use surrealdb::sql::statements::DeleteStatement;
use surrealdb::sql::statements::SelectStatement;
use surrealdb::sql::statements::UpdateStatement;
use surrealdb::sql::Array;
use surrealdb::sql::Data;
use surrealdb::sql::Field;
use surrealdb::sql::Fields;
use surrealdb::sql::Output;
use surrealdb::sql::Value;
use surrealdb::sql::Values;

/// Result type returned by the client
pub type Result<T> = std::result::Result<T, Error>;

const SUPPORTED_VERSIONS: (&str, &str) = (">=1.0.0-beta.8, <2.0.0", "20221030.c12a1cc");

/// Connection trait implemented by supported protocols
pub trait Connection: Sized + Send + Sync + 'static {
	/// The payload the caller sends to the router
	type Request: Send + Sync + Debug;
	/// The payload the router sends back to the caller
	type Response: Send + Sync + Debug;

	/// Constructs a new client without connecting to the server
	fn new(method: Method) -> Self;

	/// Connect to the server
	fn connect(
		address: ServerAddrs,
		capacity: usize,
	) -> Pin<Box<dyn Future<Output = Result<Surreal<Self>>> + Send + Sync + 'static>>;

	/// Send a query to the server
	#[allow(clippy::type_complexity)]
	fn send<'r>(
		&'r mut self,
		router: &'r Router<Self>,
		param: param::Param,
	) -> Pin<Box<dyn Future<Output = Result<Receiver<Self::Response>>> + Send + Sync + 'r>>;

	/// Receive responses for all methods except `query`
	fn recv<R>(
		&mut self,
		receiver: Receiver<Self::Response>,
	) -> Pin<Box<dyn Future<Output = Result<R>> + Send + Sync + '_>>
	where
		R: DeserializeOwned;

	/// Receive the response of the `query` method
	fn recv_query(
		&mut self,
		receiver: Receiver<Self::Response>,
	) -> Pin<Box<dyn Future<Output = Result<QueryResponse>> + Send + Sync + '_>>;

	/// Execute all methods except `query`
	fn execute<'r, R>(
		&'r mut self,
		router: &'r Router<Self>,
		param: param::Param,
	) -> Pin<Box<dyn Future<Output = Result<R>> + Send + Sync + 'r>>
	where
		R: DeserializeOwned,
	{
		Box::pin(async move {
			let rx = self.send(router, param).await?;
			self.recv(rx).await
		})
	}

	/// Execute the `query` method
	fn execute_query<'r>(
		&'r mut self,
		router: &'r Router<Self>,
		param: param::Param,
	) -> Pin<Box<dyn Future<Output = Result<QueryResponse>> + Send + Sync + 'r>> {
		Box::pin(async move {
			let rx = self.send(router, param).await?;
			self.recv_query(rx).await
		})
	}
}

/// Connect future created by `Surreal::connect`
#[derive(Debug)]
pub struct Connect<'r, C: Connection, Response> {
	router: Option<&'r OnceCell<Arc<Router<C>>>>,
	address: Result<ServerAddrs>,
	capacity: usize,
	client: PhantomData<C>,
	response_type: PhantomData<Response>,
}

impl<C, R> Connect<'_, C, R>
where
	C: Connection,
{
	/// Sets the maximum capacity of the connection
	///
	/// This is used to set bounds of the channels used internally
	/// as well set the capacity of the `HashMap` used for routing
	/// responses in case of the WebSocket client.
	///
	/// Setting this capacity to `0` (the default) means that
	/// unbounded channels will be used. If your queries per second
	/// are so high that the client is running out of memory,
	/// it might be helpful to set this to a number that works best
	/// for you.
	///
	/// # Examples
	///
	/// ```no_run
	/// # #[tokio::main]
	/// # async fn main() -> surrealdb_rs::Result<()> {
	/// use surrealdb_rs::protocol::Ws;
	/// use surrealdb_rs::Surreal;
	///
	/// let db = Surreal::connect::<Ws>("localhost:8000")
	///     .with_capacity(100_000)
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	#[must_use]
	pub const fn with_capacity(mut self, capacity: usize) -> Self {
		self.capacity = capacity;
		self
	}
}

impl<'r, Client> IntoFuture for Connect<'r, Client, Surreal<Client>>
where
	Client: Connection,
{
	type Output = Result<Surreal<Client>>;
	type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

	fn into_future(self) -> Self::IntoFuture {
		Box::pin(async move {
			let client = Client::connect(self.address?, self.capacity).await?;
			client.check_server_version();
			Ok(client)
		})
	}
}

impl<'r, Client> IntoFuture for Connect<'r, Client, ()>
where
	Client: Connection,
{
	type Output = Result<()>;
	type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'r>>;

	fn into_future(self) -> Self::IntoFuture {
		Box::pin(async move {
			match self.router {
				Some(router) => {
					let option =
						Client::connect(self.address?, self.capacity).await?.router.into_inner();
					match option {
						Some(client) => {
							let _res = router.set(client);
						}
						None => unreachable!(),
					}
				}
				None => unreachable!(),
			}
			Ok(())
		})
	}
}

#[derive(Debug)]
#[allow(dead_code)] // used by the embedded and remote connections
struct Route<A, R> {
	request: A,
	response: Sender<R>,
}

/// Message router
#[derive(Debug)]
pub struct Router<C: Connection> {
	conn: PhantomData<C>,
	sender: Sender<Option<Route<C::Request, C::Response>>>,
	#[cfg(feature = "ws")]
	last_id: AtomicI64,
}

impl<C> Router<C>
where
	C: Connection,
{
	#[cfg(feature = "ws")]
	fn next_id(&self) -> i64 {
		self.last_id.fetch_add(1, Ordering::SeqCst)
	}
}

impl<C> Drop for Router<C>
where
	C: Connection,
{
	fn drop(&mut self) {
		let _res = self.sender.send(None);
	}
}

/// `SurrealDB` client
#[derive(Debug)]
pub struct Surreal<C: Connection> {
	router: OnceCell<Arc<Router<C>>>,
}

impl<C> Surreal<C>
where
	C: Connection,
{
	fn check_server_version(&self) {
		let conn = self.clone();
		tokio::spawn(async move {
			let (versions, build_meta) = SUPPORTED_VERSIONS;
			// invalid version requirements should be caught during development
			let req = VersionReq::parse(versions).expect("valid supported versions");
			let build_meta =
				BuildMetadata::new(build_meta).expect("valid supported build metadata");
			match conn.version().await {
				Ok(version) => {
					let server_build = &version.build;
					if !req.matches(&version) {
						tracing::warn!("server version `{version}` does not match the range supported by the client `{versions}`");
					} else if !server_build.is_empty() && server_build < &build_meta {
						tracing::warn!("server build `{server_build}` is older than the minimum supported build `{build_meta}`");
					}
				}
				Err(error) => {
					tracing::trace!("failed to lookup the server version; {error:?}");
				}
			}
		});
	}
}

impl<C> Clone for Surreal<C>
where
	C: Connection,
{
	fn clone(&self) -> Self {
		Self {
			router: self.router.clone(),
		}
	}
}

/// Exposes a `connect` method for use with `Surreal::new`
pub trait StaticClient<C>
where
	C: Connection,
{
	/// Connects to a specific database endpoint, saving the connection on the static client
	fn connect<P>(&self, address: impl ToServerAddrs<P, Client = C>) -> Connect<C, ()>;
}

trait ExtractRouter<C>
where
	C: Connection,
{
	fn extract(&self) -> Result<&Router<C>>;
}

impl<C> ExtractRouter<C> for OnceCell<Arc<Router<C>>>
where
	C: Connection,
{
	fn extract(&self) -> Result<&Router<C>> {
		let router = self.get().ok_or_else(connection_uninitialised)?;
		Ok(router)
	}
}

fn connection_uninitialised() -> Error {
	ErrorKind::ConnectionUninitialized.with_message("connection uninitialized")
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn split_params(params: &mut [Value]) -> (bool, Values, Value) {
	let (what, data) = match params {
		[what] => (mem::take(what), Value::None),
		[what, data] => (mem::take(what), mem::take(data)),
		_ => unreachable!(),
	};
	let one = what.is_thing();
	let what = match what {
		Value::Array(Array(vec)) => Values(vec),
		value => Values(vec![value]),
	};
	(one, what, data)
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn create_statement(params: &mut [Value]) -> CreateStatement {
	let (_, what, data) = split_params(params);
	let data = match data {
		Value::None => None,
		value => Some(Data::ContentExpression(value)),
	};
	CreateStatement {
		what,
		data,
		output: Some(Output::After),
		..Default::default()
	}
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn update_statement(params: &mut [Value]) -> (bool, UpdateStatement) {
	let (one, what, data) = split_params(params);
	let data = match data {
		Value::None => None,
		value => Some(Data::ContentExpression(value)),
	};
	(
		one,
		UpdateStatement {
			what,
			data,
			output: Some(Output::After),
			..Default::default()
		},
	)
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn patch_statement(params: &mut [Value]) -> (bool, UpdateStatement) {
	let (one, what, data) = split_params(params);
	let data = match data {
		Value::None => None,
		value => Some(Data::PatchExpression(value)),
	};
	(
		one,
		UpdateStatement {
			what,
			data,
			output: Some(Output::Diff),
			..Default::default()
		},
	)
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn merge_statement(params: &mut [Value]) -> (bool, UpdateStatement) {
	let (one, what, data) = split_params(params);
	let data = match data {
		Value::None => None,
		value => Some(Data::MergeExpression(value)),
	};
	(
		one,
		UpdateStatement {
			what,
			data,
			output: Some(Output::After),
			..Default::default()
		},
	)
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn select_statement(params: &mut [Value]) -> (bool, SelectStatement) {
	let (one, what, _) = split_params(params);
	(
		one,
		SelectStatement {
			what,
			expr: Fields(vec![Field::All]),
			..Default::default()
		},
	)
}

#[allow(dead_code)] // used by the the embedded database and `http`
fn delete_statement(params: &mut [Value]) -> DeleteStatement {
	let (_, what, _) = split_params(params);
	DeleteStatement {
		what,
		output: Some(Output::None),
		..Default::default()
	}
}
