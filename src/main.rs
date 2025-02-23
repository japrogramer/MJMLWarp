use std::net::SocketAddr;
use std::str::FromStr;

use axum::Router;
use axum::routing::{get, post};
use clap::{Arg, Command};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod app_state;
mod handlers;
mod template_watcher;
mod utils;
mod models;

use app_state::initialize_state;
use handlers::{convert_mjml, list_templates, upload_template};

#[tokio::main]
async fn main() {
    // Initialize command-line argument parser
    let matches = Command::new("MJML Converter API")
        .version("1.0")
        .author("Your Name")
        .about("An API for converting MJML templates to HTML")
        .arg(Arg::new("log-level")
             .short('l')
             .long("log-level") // Optional in Clap 4, but good for clarity
             .value_name("LEVEL")
             .help("Sets the logging level (e.g., debug, info, warn, error)")
             .default_value("info")
             .value_parser(clap::value_parser!(String))) // Important: Use a value parser
        .get_matches();

    // Get the log level from the command-line arguments
    let log_level_str = matches.get_one::<String>("log-level").unwrap();
    let log_level = Level::from_str(log_level_str)
        .map_err(|_| format!("Invalid log level: {}", log_level_str))
        .unwrap();

    // Initialize the tracing subscriber with the specified log level
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global default subscriber");

    let app_state = initialize_state("templates").await.expect("Failed to initialize app state");
    // Creates the Axum router with routes for MJML conversion, template listing, and template upload.
    let app = Router::new()
        .route("/convert", post(convert_mjml))
        .route("/templates", get(list_templates))
        .route("/templates", post(upload_template))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));

    info!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod tests;
