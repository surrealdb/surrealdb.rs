use serde::Deserialize;
use surrealdb_rs::storage::Mem;
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

	let db = Surreal::connect::<Mem>(()).await?;

	db.use_ns("namespace").use_db("database").await?;

	let accounts: Vec<Account> = db.select(ACCOUNT).range("one".."two").await?;

	tracing::info!("{accounts:?}");

	Ok(())
}
