use surrealdb_rs::embedded::Db;
use surrealdb_rs::storage::Mem;
use surrealdb_rs::StaticClient;
use surrealdb_rs::Surreal;
use tokio::sync::mpsc;

static DB: Surreal<Db> = Surreal::new();

const NUM: usize = 100_000;

#[tokio::main]
async fn main() -> surrealdb_rs::Result<()> {
	tracing_subscriber::fmt::init();

	DB.connect::<Mem>(()).with_capacity(NUM).await?;

	DB.use_ns("namespace").use_db("database").await?;

	let (tx, mut rx) = mpsc::channel::<()>(1);

	for idx in 0..NUM {
		let sender = tx.clone();
		tokio::spawn(async move {
			#[rustfmt::skip]
            let result = DB
                .query("
                    SELECT *
                    FROM $idx
                ")
                .bind("idx", idx)
                .await
                .unwrap();

			let db_idx: Option<usize> = result.get(0, 0).unwrap();
			if let Some(db_idx) = db_idx {
				tracing::info!("{idx}: {db_idx}");
			}

			drop(sender);
		});
	}

	drop(tx);

	rx.recv().await;

	Ok(())
}
