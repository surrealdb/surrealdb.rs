use surrealdb_rs::protocol::Ws;
use surrealdb_rs::Surreal;

#[tokio::main]
async fn main() -> surrealdb_rs::Result<()> {
	tracing_subscriber::fmt::init();

	let db = Surreal::connect::<Ws>("localhost:8000").await?;

	let version = db.version().await?;

	tracing::info!("{version:?}");

	Ok(())
}
