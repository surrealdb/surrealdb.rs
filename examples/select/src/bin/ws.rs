use serde::Deserialize;
use surrealdb_rs::param::Root;
use surrealdb_rs::protocol::Ws;
use surrealdb_rs::Surreal;

const ACCOUNT: &str = "account";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Account {
	id: String,
	balance: String,
}

#[tokio::main]
async fn main() -> surrealdb_rs::Result<()> {
	tracing_subscriber::fmt::init();

	let db = Surreal::connect::<Ws>("localhost:8000").await?;

	db.signin(Root {
		username: "root",
		password: "root",
	})
	.await?;

	db.use_ns("namespace").use_db("database").await?;

	let accounts: Vec<Account> = db.select(ACCOUNT).range("one".."two").await?;

	tracing::info!("{accounts:?}");

	Ok(())
}
