//! Database storage engines

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

use crate::method::query_response::QueryResult;
use crate::param::DbResponse;
use crate::param::Param;
use crate::ErrorKind;
use crate::Method;
use crate::QueryResponse;
use crate::Result;
use crate::Route;
use std::collections::BTreeMap;
use std::mem;
#[cfg(not(target_arch = "wasm32"))]
use surrealdb::channel;
use surrealdb::sql::Array;
use surrealdb::sql::Query;
use surrealdb::sql::Statement;
use surrealdb::sql::Statements;
use surrealdb::sql::Strand;
use surrealdb::sql::Value;
use surrealdb::Datastore;
use surrealdb::Response;
use surrealdb::Session;
#[cfg(not(target_arch = "wasm32"))]
use tokio::fs::OpenOptions;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncReadExt;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncWriteExt;

type DbRoute = Route<(Method, Param), Result<DbResponse>>;

/// In-memory database
///
/// # Examples
///
/// Instantiating a global instance
///
/// ```
/// use surrealdb_rs::{Result, Surreal};
/// use surrealdb_rs::embedded::Db;
/// use surrealdb_rs::storage::Mem;
/// use surrealdb_rs::StaticClient;
///
/// static DB: Surreal<Db> = Surreal::new();
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     DB.connect::<Mem>(()).await?;
///
///     Ok(())
/// }
/// ```
///
/// Instantiating an in-memory instance
///
/// ```
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::Mem;
///
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// let db = Surreal::connect::<Mem>(()).await?;
/// # Ok(())
/// # }
/// ```
///
/// Instantiating an in-memory strict instance
///
/// ```
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::Mem;
///
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// let db = Surreal::connect::<Mem>(Strict).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "mem")]
#[cfg_attr(docsrs, doc(cfg(feature = "mem")))]
#[derive(Debug)]
pub struct Mem;

/// File database
///
/// # Examples
///
/// Instantiating a file-backed instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::File;
///
/// let db = Surreal::connect::<File>("temp.db").await?;
/// # Ok(())
/// # }
/// ```
///
/// Instantiating a file-backed strict instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::File;
///
/// let db = Surreal::connect::<File>(("temp.db", Strict)).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "rocksdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "rocksdb")))]
#[derive(Debug)]
pub struct File;

/// RocksDB database
///
/// # Examples
///
/// Instantiating a RocksDB-backed instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::RocksDb;
///
/// let db = Surreal::connect::<RocksDb>("temp.db").await?;
/// # Ok(())
/// # }
/// ```
///
/// Instantiating a RocksDB-backed strict instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::RocksDb;
///
/// let db = Surreal::connect::<RocksDb>(("temp.db", Strict)).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "rocksdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "rocksdb")))]
#[derive(Debug)]
pub struct RocksDb;

/// IndxDB database
///
/// # Examples
///
/// Instantiating a IndxDB-backed instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::IndxDb;
///
/// let db = Surreal::connect::<IndxDb>("MyDatabase").await?;
/// # Ok(())
/// # }
/// ```
///
/// Instantiating a IndxDB-backed strict instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::IndxDb;
///
/// let db = Surreal::connect::<IndxDb>(("MyDatabase", Strict)).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "indxdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "indxdb")))]
#[derive(Debug)]
pub struct IndxDb;

/// TiKV database
///
/// # Examples
///
/// Instantiating a TiKV instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::TiKv;
///
/// let db = Surreal::connect::<TiKv>("127.0.0.1:2379").await?;
/// # Ok(())
/// # }
/// ```
///
/// Instantiating a TiKV strict instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::TiKv;
///
/// let db = Surreal::connect::<TiKv>(("127.0.0.1:2379", Strict)).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tikv")]
#[cfg_attr(docsrs, doc(cfg(feature = "tikv")))]
#[derive(Debug)]
pub struct TiKv;

/// FoundationDB database
///
/// # Examples
///
/// Instantiating a FoundationDB-backed instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::FDb;
///
/// let db = Surreal::connect::<FDb>("fdb.cluster").await?;
/// # Ok(())
/// # }
/// ```
///
/// Instantiating a FoundationDB-backed strict instance
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() -> surrealdb_rs::Result<()> {
/// use surrealdb_rs::param::Strict;
/// use surrealdb_rs::Surreal;
/// use surrealdb_rs::storage::FDb;
///
/// let db = Surreal::connect::<FDb>(("fdb.cluster", Strict)).await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "fdb")]
#[cfg_attr(docsrs, doc(cfg(feature = "fdb")))]
#[derive(Debug)]
pub struct FDb;

fn process(responses: Vec<Response>) -> Result<QueryResponse> {
	let mut vec = Vec::with_capacity(responses.len());
	for response in responses {
		match response.result {
			Ok(value) => match value {
				Value::Array(Array(array)) => vec.push(QueryResult(Ok(array))),
				Value::None | Value::Null => vec.push(QueryResult(Ok(vec![]))),
				value => vec.push(QueryResult(Ok(vec![value]))),
			},
			Err(error) => vec.push(QueryResult(Err(ErrorKind::Query.with_context(error)))),
		}
	}
	Ok(vec.into())
}

async fn take(one: bool, responses: Vec<Response>) -> Result<Value> {
	if let Some(result) = process(responses)?.into_inner().pop() {
		let mut vec = result.into_inner()?;
		match one {
			true => match vec.pop() {
				Some(Value::Array(Array(mut vec))) => {
					if let [value] = &mut vec[..] {
						return Ok(mem::take(value));
					}
				}
				Some(Value::None | Value::Null) | None => {}
				Some(value) => {
					return Ok(value);
				}
			},
			false => {
				return Ok(Value::Array(Array(vec)));
			}
		}
	}
	match one {
		true => Ok(Value::None),
		false => Ok(Value::Array(Array(vec![]))),
	}
}

async fn router(
	(method, param): (Method, Param),
	#[cfg(target_arch = "wasm32")] kvs: &Datastore,
	#[cfg(not(target_arch = "wasm32"))] kvs: &'static Datastore,
	session: &mut Session,
	vars: &mut BTreeMap<String, Value>,
	strict: bool,
) -> Result<DbResponse> {
	let mut params = param.other;

	match method {
		Method::Use => {
			let (ns, db) = match &mut params[..] {
				[Value::Strand(Strand(ns)), Value::Strand(Strand(db))] => {
					(mem::take(ns), mem::take(db))
				}
				_ => unreachable!(),
			};
			session.ns = Some(ns);
			session.db = Some(db);
			Ok(DbResponse::Other(Value::None))
		}
		Method::Signin | Method::Signup | Method::Authenticate | Method::Invalidate => {
			unreachable!()
		}
		Method::Create => {
			let statement = crate::create_statement(&mut params);
			let query = Query(Statements(vec![Statement::Create(statement)]));
			let response = kvs.process(query, &*session, Some(vars.clone()), strict).await?;
			let value = take(true, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Update => {
			let (one, statement) = crate::update_statement(&mut params);
			let query = Query(Statements(vec![Statement::Update(statement)]));
			let response = kvs.process(query, &*session, Some(vars.clone()), strict).await?;
			let value = take(one, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Patch => {
			let (one, statement) = crate::patch_statement(&mut params);
			let query = Query(Statements(vec![Statement::Update(statement)]));
			let response = kvs.process(query, &*session, Some(vars.clone()), strict).await?;
			let value = take(one, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Merge => {
			let (one, statement) = crate::merge_statement(&mut params);
			let query = Query(Statements(vec![Statement::Update(statement)]));
			let response = kvs.process(query, &*session, Some(vars.clone()), strict).await?;
			let value = take(one, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Select => {
			let (one, statement) = crate::select_statement(&mut params);
			let query = Query(Statements(vec![Statement::Select(statement)]));
			let response = kvs.process(query, &*session, Some(vars.clone()), strict).await?;
			let value = take(one, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Delete => {
			let statement = crate::delete_statement(&mut params);
			let query = Query(Statements(vec![Statement::Delete(statement)]));
			let response = kvs.process(query, &*session, Some(vars.clone()), strict).await?;
			let value = take(true, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Query => {
			let response = match param.query {
				Some((query, mut bindings)) => {
					let mut vars = vars.clone();
					vars.append(&mut bindings);
					kvs.process(query, &*session, Some(vars), strict).await?
				}
				None => unreachable!(),
			};
			let response = process(response)?;
			Ok(DbResponse::Query(response))
		}
		#[cfg(target_arch = "wasm32")]
		Method::Export | Method::Import => unreachable!(),
		#[cfg(not(target_arch = "wasm32"))]
		Method::Export => {
			let file = param.file.expect("file to export into");
			let (tx, rx) = channel::new(1);
			let ns = session.ns.clone().unwrap_or_default();
			let db = session.db.clone().unwrap_or_default();
			tokio::spawn(async move {
				if let Err(error) = kvs.export(ns, db, tx).await {
					tracing::error!("{error}");
				}
			});
			let (mut writer, mut reader) = io::duplex(10_240);
			tokio::spawn(async move {
				while let Ok(value) = rx.recv().await {
					if let Err(error) = writer.write_all(&value).await {
						tracing::error!("{error}");
					}
				}
			});
			let mut file =
				OpenOptions::new().write(true).create(true).truncate(true).open(file).await?;
			io::copy(&mut reader, &mut file).await?;
			Ok(DbResponse::Other(Value::None))
		}
		#[cfg(not(target_arch = "wasm32"))]
		Method::Import => {
			let file = param.file.expect("file to import from");
			let mut file = OpenOptions::new().read(true).open(file).await?;
			let mut statements = String::new();
			file.read_to_string(&mut statements).await?;
			let responses = kvs.execute(&statements, &*session, Some(vars.clone()), strict).await?;
			for response in responses {
				response.result?;
			}
			Ok(DbResponse::Other(Value::None))
		}
		Method::Health => Ok(DbResponse::Other(Value::None)),
		Method::Version => Ok(DbResponse::Other(surrealdb::VERSION.into())),
		Method::Set => {
			let (key, value) = match &mut params[..2] {
				[Value::Strand(Strand(key)), value] => (mem::take(key), mem::take(value)),
				_ => unreachable!(),
			};
			vars.insert(key, value);
			Ok(DbResponse::Other(Value::None))
		}
		Method::Unset => {
			if let [Value::Strand(Strand(key))] = &params[..1] {
				vars.remove(key);
			}
			Ok(DbResponse::Other(Value::None))
		}
		Method::Live => {
			let table = match &mut params[..] {
				[value] => mem::take(value),
				_ => unreachable!(),
			};
			let mut vars = BTreeMap::new();
			vars.insert("table".to_owned(), table);
			let response = kvs
				.execute("LIVE SELECT * FROM type::table($table)", &*session, Some(vars), strict)
				.await?;
			let value = take(true, response).await?;
			Ok(DbResponse::Other(value))
		}
		Method::Kill => {
			let id = match &mut params[..] {
				[value] => mem::take(value),
				_ => unreachable!(),
			};
			let mut vars = BTreeMap::new();
			vars.insert("id".to_owned(), id);
			let response =
				kvs.execute("KILL type::string($id)", &*session, Some(vars), strict).await?;
			let value = take(true, response).await?;
			Ok(DbResponse::Other(value))
		}
	}
}
