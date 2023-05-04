use crate::metrics::Metrics;
use axum::{extract::State, routing::get, Router};
use clap::Parser;
use std::{net::SocketAddr, sync::Arc};
mod metrics;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let _metrics_server = tokio::join!(start_metrics_server());
    Ok(())
}

async fn start_metrics_server() {
    let args = Args::parse();
    let app = metrics_app(&args);
    let addr: SocketAddr = args.socket_address.parse().unwrap();
    println!("Using log path: {:?}", args.log_path);
    println!("Using metric prefix: {:?}", args.prefix);
    println!("Listening on http://{:?}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn metrics_app(args: &Args) -> Router {
    let metrics = Arc::new(Metrics::new(args.log_path.clone(), args.prefix.clone()));
    Router::new()
        .route("/metrics", get(handler))
        .with_state(metrics)
}

async fn handler(State(state): State<Arc<Metrics>>) -> String {
    state.record_metrics();
    state.render().await
}

/// Nginx Log File Prometheus Exporter
/// - Parses the newest log file lines every prometheus scrape and provides a /metrics endpoint for collection
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Access Log file path
    #[arg(short, long, default_value = "/var/log/nginx/access.log")]
    log_path: String,

    /// Address and Port to serve the app on
    #[arg(short, long, default_value = "0.0.0.0:9200")]
    socket_address: String,

    /// Prefix appended to every metric (useful for filtering)
    #[arg(short, long, default_value = "")]
    prefix: String,
}
