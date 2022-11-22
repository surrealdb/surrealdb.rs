#![cfg(feature = "http")]
#![cfg(not(target_arch = "wasm32"))]

mod types;

use crate::types::AuthParams;
use serde_json::json;
use std::ops::Bound;
use surrealdb::sql::statements::BeginStatement;
use surrealdb::sql::statements::CommitStatement;
use surrealdb_rs::param::Database;
use surrealdb_rs::param::Jwt;
use surrealdb_rs::param::NameSpace;
use surrealdb_rs::param::PatchOp;
use surrealdb_rs::param::Root;
use surrealdb_rs::param::Scope;
use surrealdb_rs::protocol::Http;
use surrealdb_rs::Surreal;
use tokio::fs::remove_file;
use types::*;
use ulid::Ulid;

#[tokio::test]
async fn connect() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.health().await.unwrap();
}

#[tokio::test]
async fn connect_with_capacity() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).with_capacity(512).await.unwrap();
	db.health().await.unwrap();
}

#[tokio::test]
async fn invalidate() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.invalidate().await.unwrap();
	match db.create("user").await {
		Ok(result) => {
			let option: Option<RecordId> = result;
			assert!(option.is_none());
		}
		Err(error) => {
			assert!(error
				.to_string()
				.contains("You don't have permission to perform this query type"));
		}
	}
}

#[tokio::test]
async fn yuse() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db("database").await.unwrap();
}

#[tokio::test]
async fn signup_scope() {
	let database = Ulid::new().to_string();
	let scope = Ulid::new().to_string();
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(&database).await.unwrap();
	#[rustfmt::skip]
    db.query(format!("
        DEFINE SCOPE {scope} SESSION 1s
        SIGNUP ( CREATE user SET email = $email, pass = crypto::argon2::generate($pass) )
        SIGNIN ( SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(pass, $pass) )
    ;"))
    .await
    .unwrap();
	db.signup(Scope {
		namespace: NS,
		database: &database,
		scope: &scope,
		params: AuthParams {
			email: "john.doe@example.com",
			pass: "password123",
		},
	})
	.await
	.unwrap();
}

#[tokio::test]
async fn signin_root() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
}

#[tokio::test]
async fn signin_ns() {
	let user = Ulid::new().to_string();
	let pass = "password123";
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	#[rustfmt::skip]
    db.query(format!("
        DEFINE LOGIN {user} ON NAMESPACE PASSWORD '{pass}'
    "))
    .await
    .unwrap();
	let _: Jwt = db
		.signin(NameSpace {
			namespace: NS,
			username: &user,
			password: pass,
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn signin_db() {
	let database = Ulid::new().to_string();
	let user = Ulid::new().to_string();
	let pass = "password123";
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(&database).await.unwrap();
	#[rustfmt::skip]
    db.query(format!("
        DEFINE LOGIN {user} ON DATABASE PASSWORD '{pass}'
    "))
    .await
    .unwrap();
	let _: Jwt = db
		.signin(Database {
			namespace: NS,
			database: &database,
			username: &user,
			password: pass,
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn signin_scope() {
	let database = Ulid::new().to_string();
	let scope = Ulid::new().to_string();
	let email = format!("{scope}@example.com");
	let pass = "password123";
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(&database).await.unwrap();
	#[rustfmt::skip]
    db.query(format!("
        DEFINE SCOPE {scope} SESSION 1s
        SIGNUP ( CREATE user SET email = $email, pass = crypto::argon2::generate($pass) )
        SIGNIN ( SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(pass, $pass) )
    ;"))
    .await
    .unwrap();
	db.signup(Scope {
		namespace: NS,
		database: &database,
		scope: &scope,
		params: AuthParams {
			pass,
			email: &email,
		},
	})
	.await
	.unwrap();

	db.signin(Scope {
		namespace: NS,
		database: &database,
		scope: &scope,
		params: AuthParams {
			pass,
			email: &email,
		},
	})
	.await
	.unwrap();
}

#[tokio::test]
async fn authenticate() {
	let user = Ulid::new().to_string();
	let pass = "password123";
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	#[rustfmt::skip]
    db.query(format!("
        DEFINE LOGIN {user} ON NAMESPACE PASSWORD '{pass}'
    "))
    .await
    .unwrap();
	let token = db
		.signin(NameSpace {
			namespace: NS,
			username: &user,
			password: pass,
		})
		.await
		.unwrap();
	db.authenticate(token).await.unwrap();
}

#[tokio::test]
async fn query() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.query("SELECT * FROM record").await.unwrap();
}

#[tokio::test]
async fn query_binds() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.query("CREATE user:john SET name = $name").bind("name", "John Doe").await.unwrap();
}

#[tokio::test]
async fn query_chaining() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create("user").await.unwrap();
}

#[tokio::test]
async fn create_record_with_id() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create(("user", "john")).await.unwrap();
}

#[tokio::test]
async fn create_record_no_id_with_content() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: Vec<RecordId> = db.select(table).await.unwrap();
}

#[tokio::test]
async fn select_record_id() {
	let (table, id) = ("user", "john");
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: RecordId = db.create((table, id)).await.unwrap();
	let _: RecordId = db.create(table).await.unwrap();
	let _: Option<RecordId> = db.select((table, id)).await.unwrap();
}

#[tokio::test]
async fn select_record_ranges() {
	let table = "user";
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	let _: Vec<RecordId> = db.update("user").await.unwrap();
}

#[tokio::test]
async fn update_record_id() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	let _: Option<RecordId> = db.update(("user", "john")).await.unwrap();
}

#[tokio::test]
async fn update_table_with_content() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.delete("user").await.unwrap();
}

#[tokio::test]
async fn delete_record_id() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.delete(("user", "john")).await.unwrap();
}

#[tokio::test]
async fn delete_record_range() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(Ulid::new().to_string()).await.unwrap();
	db.delete("user").range("jane".."john").await.unwrap();
}

#[tokio::test]
async fn version() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.version().await.unwrap();
}

#[tokio::test]
async fn set_unset() {
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.use_ns(NS).use_db(&db_name).await.unwrap();
	db.signin(Root {
		username: ROOT_USER,
		password: ROOT_PASS,
	})
	.await
	.unwrap();
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
	let db = Surreal::connect::<Http>(DB_ENDPOINT).await.unwrap();
	db.query("RETURN true").await.unwrap();
}
