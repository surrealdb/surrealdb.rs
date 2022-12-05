#![cfg(any(feature = "ws", feature = "http"))]

use serde::Deserialize;
use serde::Serialize;
use surrealdb::sql::Uuid;
use surrealdb_rs::param::Root;
use surrealdb_rs::param::ToServerAddrs;
use surrealdb_rs::Surreal;

pub const NS: &str = "test-ns";
pub const DB: &str = "test-db";
pub const ROOT_USER: &str = "root";
pub const ROOT_PASS: &str = "root";
pub const DB_ENDPOINT: &str = "localhost:8000";

#[derive(Debug, Serialize)]
pub struct Record<'a> {
	pub name: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct RecordId {
	pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthParams<'a> {
	pub email: &'a str,
	pub pass: &'a str,
}

/// Returns a sort of "sandboxed" client in the way it uses a completely random
/// namespace/database pair to ensure the data that is pushed & retrieved from/to
/// that client is strictly unique to it and won't interfere with other tests.
pub async fn sandboxed_client<'a, Protocol>(
) -> Surreal<<&'a str as ToServerAddrs<Protocol>>::Client>
where
	&'a str: ToServerAddrs<Protocol>,
{
	let client = Surreal::connect::<Protocol>(DB_ENDPOINT).await.unwrap();
	client
		.signin(Root {
			username: ROOT_USER,
			password: ROOT_PASS,
		})
		.await
		.unwrap();

	// use
	let _: () =
		client.use_ns(Uuid::new().to_string()).use_db(Uuid::new().to_string()).await.unwrap();

	client
}
