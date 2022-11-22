#![cfg(feature = "mem")]
#![cfg(not(target_arch = "wasm32"))]

mod types;

use serde_json::json;
use std::ops::Bound;
use surrealdb::sql::statements::BeginStatement;
use surrealdb::sql::statements::CommitStatement;
use surrealdb_rs::param::PatchOp;
use surrealdb_rs::storage::Mem;
use surrealdb_rs::Surreal;
use tokio::fs::remove_file;
use types::*;
use ulid::Ulid;

#[tokio::test]
async fn connect() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.health().await.unwrap();
}

#[tokio::test]
async fn connect_with_capacity() {
	let db = Surreal::connect::<Mem>(()).with_capacity(512).await.unwrap();
	db.health().await.unwrap();
}

#[tokio::test]
async fn yuse() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
}

#[tokio::test]
async fn query() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.query("SELECT * FROM record").await.unwrap();
}

#[tokio::test]
async fn query_binds() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.query("CREATE user:john SET name = $name").bind("name", "John Doe").await.unwrap();
}

#[tokio::test]
async fn query_chaining() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.query(BeginStatement)
		.query("CREATE account:one SET balance = 135605.16")
		.query("CREATE account:two SET balance = 91031.31")
		.query("UPDATE account:one SET balance += 300.00")
		.query("UPDATE account:two SET balance -= 300.00")
		.query(CommitStatement)
		.await
		.unwrap();
}

#[tokio::test]
async fn create_record_no_id() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create("user").await.unwrap();
}

#[tokio::test]
async fn create_record_with_id() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create(("user", "john")).await.unwrap();
}

#[tokio::test]
async fn create_record_no_id_with_content() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db
		.create("user")
		.content(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn create_record_with_id_with_content() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let record: RecordId = db
		.create(("user", "john"))
		.content(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
	assert_eq!(record.id, format!("user:john"));
}

#[tokio::test]
async fn select_table() {
	let table = "user";
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: Vec<RecordId> = db.select(table).await.unwrap();
}

#[tokio::test]
async fn select_record_id() {
	let (table, id) = ("user", "john");
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create((table, id)).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: Option<RecordId> = db.select((table, id)).await.unwrap();
}

#[tokio::test]
async fn select_record_ranges() {
	let table = "user";
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create((table, "amos")).await.unwrap();
	let _: RecordId = db.create((table, "jane")).await.unwrap();
	let _: RecordId = db.create((table, "john")).await.unwrap();
	let _: RecordId = db.create((table, "zoey")).await.unwrap();
	let convert = |users: Vec<RecordId>| -> Vec<String> {
		users
			.into_iter()
			.map(|user| user.id.trim_start_matches(&format!("{table}:")).to_owned())
			.collect()
	};
	let users: Vec<RecordId> = db.select(table).range(..).await.unwrap();
	assert_eq!(convert(users), vec!["amos", "jane", "john", "zoey"]);
	let users: Vec<RecordId> = db.select(table).range(.."john").await.unwrap();
	assert_eq!(convert(users), vec!["amos", "jane"]);
	let users: Vec<RecordId> = db.select(table).range(..="john").await.unwrap();
	assert_eq!(convert(users), vec!["amos", "jane", "john"]);
	let users: Vec<RecordId> = db.select(table).range("jane"..).await.unwrap();
	assert_eq!(convert(users), vec!["jane", "john", "zoey"]);
	let users: Vec<RecordId> = db.select(table).range("jane".."john").await.unwrap();
	assert_eq!(convert(users), vec!["jane"]);
	let users: Vec<RecordId> = db.select(table).range("jane"..="john").await.unwrap();
	assert_eq!(convert(users), vec!["jane", "john"]);
	let users: Vec<RecordId> =
		db.select(table).range((Bound::Excluded("jane"), Bound::Included("john"))).await.unwrap();
	assert_eq!(convert(users), vec!["john"]);
}

#[tokio::test]
async fn update_table() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db.update("user").await.unwrap();
}

#[tokio::test]
async fn update_record_id() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Option<RecordId> = db.update(("user", "john")).await.unwrap();
}

#[tokio::test]
async fn update_table_with_content() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db
		.update("user")
		.content(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn update_record_range_with_content() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db
		.update("user")
		.range("jane".."john")
		.content(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn update_record_id_with_content() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Option<RecordId> = db
		.update(("user", "john"))
		.content(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn merge_table() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db
		.update("user")
		.merge(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn merge_record_range() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db
		.update("user")
		.range("jane".."john")
		.merge(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn merge_record_id() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Option<RecordId> = db
		.update(("user", "john"))
		.merge(Record {
			name: "John Doe",
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn patch_table() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db
		.update("user")
		.patch(PatchOp::replace("/baz", "boo"))
		.patch(PatchOp::add("/hello", ["world"]))
		.patch(PatchOp::remove("/foo"))
		.await
		.unwrap();
}

#[tokio::test]
async fn patch_record_range() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db
		.update("user")
		.range("jane".."john")
		.patch(PatchOp::replace("/baz", "boo"))
		.patch(PatchOp::add("/hello", ["world"]))
		.patch(PatchOp::remove("/foo"))
		.await
		.unwrap();
}

#[tokio::test]
async fn patch_record_id() {
	let id = "record";
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Option<RecordId> = db
		.create(("user", id))
		.content(json!({
			"baz": "qux",
			"foo": "bar"
		}))
		.await
		.unwrap();
	let _: Option<serde_json::Value> = db
		.update(("user", id))
		.patch(PatchOp::replace("/baz", "boo"))
		.patch(PatchOp::add("/hello", ["world"]))
		.patch(PatchOp::remove("/foo"))
		.await
		.unwrap();
	let value: Option<serde_json::Value> = db.select(("user", id)).await.unwrap();
	assert_eq!(
		value,
		Some(json!({
			"id": format!("user:{id}"),
			"baz": "boo",
			"hello": ["world"]
		}))
	);
}

#[tokio::test]
async fn delete_table() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.delete("user").await.unwrap();
}

#[tokio::test]
async fn delete_record_id() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.delete(("user", "john")).await.unwrap();
}

#[tokio::test]
async fn delete_record_range() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.delete("user").range("jane".."john").await.unwrap();
}

#[tokio::test]
async fn version() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.version().await.unwrap();
}

#[tokio::test]
async fn set_unset() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.set(
		"user",
		Record {
			name: "John Doe",
		},
	)
	.await
	.unwrap();
	db.unset("user").await.unwrap();
}

#[tokio::test]
async fn export_import() {
	let db_name = Ulid::new().to_string();
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.use_ns(NS).use_db(&db_name).await.unwrap();
	for i in 0..10 {
		let _: RecordId = db
			.create("user")
			.content(Record {
				name: &format!("User {i}"),
			})
			.await
			.unwrap();
	}
	let file = format!("{db_name}.sql");
	db.export(&file).await.unwrap();
	db.import(&file).await.unwrap();
	remove_file(file).await.unwrap();
}

#[tokio::test]
async fn return_bool() {
	let db = Surreal::connect::<Mem>(()).await.unwrap();
	db.query("RETURN true").await.unwrap();
}
