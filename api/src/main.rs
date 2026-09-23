use std::net::SocketAddr;

use dataroom_api::{app, db};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("API_PORT").unwrap_or_else(|_| "4318".into());
    let url =
        std::env::var("DATABASE_URL").expect("Set DATABASE_URL or run make local / make dev.");
    let pool = db::connect(&url)
        .await
        .map_err(|_| "Could not connect to PostgreSQL.")?;
    let result: Result<(), Box<dyn std::error::Error>> = async {
        db::initialize(&pool)
            .await
            .map_err(|_| "Database migration failed.")?;
        let address: SocketAddr = format!("{host}:{port}").parse()?;
        let listener = tokio::net::TcpListener::bind(address).await?;
        println!("Dataroom API: http://{address}");
        axum::serve(listener, app(pool.clone()))
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    }
    .await;
    pool.close().await;
    result?;
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = async { tokio::signal::ctrl_c().await.expect("interrupt handler") };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("terminate handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = interrupt => {},
        _ = terminate => {},
    }
}
