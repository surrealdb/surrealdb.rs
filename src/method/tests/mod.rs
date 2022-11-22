#![cfg(any(feature = "http", feature = "ws"))]
#![cfg(not(target_arch = "wasm32"))]

mod protocol;
mod server;
mod types;

use crate::param::PatchOp;
use crate::QueryResponse;
use crate::StaticClient;
use crate::Surreal;
use protocol::Client;
use protocol::Test;
use semver::Version;
use std::ops::Bound;
use surrealdb::sql::statements::BeginStatement;
use surrealdb::sql::statements::CommitStatement;
use types::User;
use types::USER;

static DB: Surreal<Client> = Surreal::new();

#[tokio::test]
async fn api() {
	// connect to the mock server
	DB.connect::<Test>(()).with_capacity(512).await.unwrap();

	// health
	let _: () = DB.health().await.unwrap();

	// use
	let _: () = DB.use_ns("test-ns").use_db("test-db").await.unwrap();

	// query
	let _: QueryResponse = DB.query("SELECT * FROM user").await.unwrap();
	let _: QueryResponse =
		DB.query("CREATE user:john SET name = $name").bind("name", "John Doe").await.unwrap();
	let _: QueryResponse = DB
		.query(BeginStatement)
		.query("CREATE account:one SET balance = 135605.16")
		.query("CREATE account:two SET balance = 91031.31")
		.query("UPDATE account:one SET balance += 300.00")
		.query("UPDATE account:two SET balance -= 300.00")
		.query(CommitStatement)
		.await
		.unwrap();

	// create
	let _: User = DB.create(USER).await.unwrap();
	let _: User = DB.create((USER, "john")).await.unwrap();
	let _: User = DB.create(USER).content(User::default()).await.unwrap();
	let _: User = DB.create((USER, "john")).content(User::default()).await.unwrap();

	// select
	let _: Vec<User> = DB.select(USER).await.unwrap();
	let _: Option<User> = DB.select((USER, "john")).await.unwrap();
	let _: Vec<User> = DB.select(USER).range(..).await.unwrap();
	let _: Vec<User> = DB.select(USER).range(.."john").await.unwrap();
	let _: Vec<User> = DB.select(USER).range(..="john").await.unwrap();
	let _: Vec<User> = DB.select(USER).range("jane"..).await.unwrap();
	let _: Vec<User> = DB.select(USER).range("jane".."john").await.unwrap();
	let _: Vec<User> = DB.select(USER).range("jane"..="john").await.unwrap();
	let _: Vec<User> = DB.select(USER).range("jane"..="john").await.unwrap();
	let _: Vec<User> =
		DB.select(USER).range((Bound::Excluded("jane"), Bound::Included("john"))).await.unwrap();

	// update
	let _: Vec<User> = DB.update(USER).await.unwrap();
	let _: Option<User> = DB.update((USER, "john")).await.unwrap();
	let _: Vec<User> = DB.update(USER).content(User::default()).await.unwrap();
	let _: Vec<User> =
		DB.update(USER).range("jane".."john").content(User::default()).await.unwrap();
	let _: Option<User> = DB.update((USER, "john")).content(User::default()).await.unwrap();

	// merge
	let _: Vec<User> = DB.update(USER).merge(User::default()).await.unwrap();
	let _: Vec<User> = DB.update(USER).range("jane".."john").merge(User::default()).await.unwrap();
	let _: Option<User> = DB.update((USER, "john")).merge(User::default()).await.unwrap();

	// patch
	let _: Vec<User> = DB.update(USER).patch(PatchOp::remove("/name")).await.unwrap();
	let _: Vec<User> =
		DB.update(USER).range("jane".."john").patch(PatchOp::remove("/name")).await.unwrap();
	let _: Option<User> = DB.update((USER, "john")).patch(PatchOp::remove("/name")).await.unwrap();

	// delete
	let _: () = DB.delete(USER).await.unwrap();
	let _: () = DB.delete((USER, "john")).await.unwrap();
	let _: () = DB.delete(USER).range("jane".."john").await.unwrap();

	// version
	let _: Version = DB.version().await.unwrap();
}

fn send_and_sync(_: impl Send + Sync) {}

#[test]
fn futures_are_send_and_sync() {
	send_and_sync(async {
		let db = Surreal::connect::<Test>(()).await.unwrap();
		db.use_ns("test-ns").use_db("test-db").await.unwrap();
		let _: Vec<User> = db.select(USER).await.unwrap();
	});
}
