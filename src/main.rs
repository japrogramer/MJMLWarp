use axum::{
    extract::{Json, Multipart, State},
    http::{StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use handlebars::Handlebars;
use mrml::{prelude::render::RenderOptions, self};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use lru::LruCache;
use tokio::time::interval;
use tokio::io::{AsyncWriteExt};
use tokio::io::BufWriter as AsyncBufWriter;
use tokio::fs::File;
use std::fs::read_to_string;
use tokio::sync::RwLock; // For concurrent access to shared state

struct CachedTemplate {
    content: String,
    last_accessed: Instant,
}

// Define a struct to hold application state, including the Handlebars registry
#[derive(Clone)]
struct AppState {
    handlebars: Handlebars<'static>,
    template_cache: Arc<RwLock<LruCache<String, CachedTemplate>>>,
}

impl AppState {
    fn new(cache_capacity: usize) -> Self {
        AppState {
            handlebars: Handlebars::new(),
            template_cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(cache_capacity).unwrap()))),
        }
    }

    async fn get_template(&self, path: &str) -> Result<String, String> {
        let mut cache = self.template_cache.write().await;
        if let Some(cached) = cache.get_mut(path) {
            cached.last_accessed = Instant::now();
            Ok(cached.content.clone())
        } else {
            // Load the template from disk
            let template_content = read_to_string(path)
                .map_err(|e| format!("Failed to read template file {}: {}", path, e))?;
            // Store the template in the cache
            cache.put(path.to_string(), CachedTemplate {
                content: template_content.clone(), // Store a clone of the content
                last_accessed: Instant::now(),
            });
            println!("New Template cached.  {} templates cached.", cache.len());
            Ok(template_content) // Return the template content
        }
    }

    async fn insert_template(&self, path: String, content: String) {
        let mut cache = self.template_cache.write().await;
        cache.put(path, CachedTemplate {
            content,
            last_accessed: Instant::now(),
        });
    }

    async fn clean_old_templates(&self, max_age: Duration) {
        let mut cache = self.template_cache.write().await;
        let now = Instant::now(); // Capture the current time

        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter_map(|(key, template)| {
                if now.duration_since(template.last_accessed) >= max_age {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            cache.demote(&key);
        }
        println!("Template cache cleaned.  {} templates cached.", cache.len());
    }
}

#[derive(Deserialize)]
struct MjmlInput {
    mjml: Option<String>,
    payload: Value,
    template: Option<String>,
}

async fn convert_mjml(
    State(app_state): State<AppState>,
    Json(payload): Json<MjmlInput>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let mjml_content = match &payload.template {
        Some(template_name) => {
            let template_path = format!("./templates/{}", template_name);
            app_state
                .get_template(&template_path)
                .await
                .map_err(|e| {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to read template file {}: {}", template_path, e),
                    )
                })?
        }
        None => payload
            .mjml
            .clone()
            .ok_or((axum::http::StatusCode::BAD_REQUEST, "Missing MJML input".to_string()))?,
    };

    let mjml_content = app_state
        .handlebars
        .render_template(&mjml_content, &payload.payload)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Handlebars rendering error: {}", e),
            )
        })?;
    let parsed = mrml::parse(&mjml_content).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid MJML input: {}", e),
        )
    })?;
    let rendered = parsed.render(&RenderOptions::default()).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Couldn't render MJML template: {}", e),
        )
    })?;
    Ok((axum::http::StatusCode::OK, rendered).into_response())
}

/// Lists all MJML templates in the ./templates directory.
async fn list_templates() -> Result<impl IntoResponse, (StatusCode, String)> {
    let templates_dir = Path::new("./templates");
    let entries = match fs::read_dir(templates_dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read templates directory: {}", e),
            ))
        }
    };

    let templates: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect();

    Ok(Json(templates))
}

/// Uploads a new MJML template to the ./templates directory. Validates file type and MJML syntax.
async fn upload_template(
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let mut templates = Vec::new();

    while let Ok(Some(mut field)) = multipart.next_field().await { // Added mut here
        let file_name = match field.file_name() {
            Some(name) => name.to_owned(),
            None => {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    "Missing filename".to_string(),
                ))
            }
        };

        let content_type = field.content_type().unwrap_or("text/plain").to_string();

        if content_type != "text/plain" {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid file type. Only text/plain is allowed.".to_string(),
            ));
        }

        let mut buffer: Vec<u8> = Vec::with_capacity(8192);
        while let Ok(Some(chunk)) = field.chunk().await {
            buffer.extend_from_slice(&chunk);
        }

        match String::from_utf8(buffer) {
            Ok(mjml_content) => {
                let parsed = mrml::parse(&mjml_content);

                match parsed {
                    Ok(_) => {
                        let file_path = format!("./templates/{}", file_name);
                        let file = File::create(file_path)
                            .await
                            .map_err(|e| {
                                (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Failed to create file: {}", e),
                                )
                            })?;
                        let mut buffer_writer = AsyncBufWriter::new(file);
                        buffer_writer
                            .write_all(mjml_content.as_bytes())
                            .await
                            .map_err(|e| {
                                (
                                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Failed to write file: {}", e),
                                )
                            })?;
                        buffer_writer.flush().await.map_err(|e| {
                            (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to flush file: {}", e),
                            )
                        })?;
                        templates.push(file_name);
                    }
                    Err(_) => {
                        return Err((
                            axum::http::StatusCode::BAD_REQUEST,
                            "Invalid MJML input".to_string(),
                        ))
                    }
                }
            }
            Err(e) => {
                eprintln!("Invalid UTF-8 sequence: {}", e);
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    "Invalid UTF-8 encoding".to_string(),
                ));
            }
        }
    }

    if templates.is_empty() {
        Ok((axum::http::StatusCode::OK, "No files uploaded".to_string()))
    } else {
        Ok((
            axum::http::StatusCode::OK,
            format!("Templates {:?} uploaded successfully", templates),
        ))
    }
}

// Function to clean the template cache
async fn clean_template_cache(app_state: &AppState) {
    let expiration_duration = Duration::from_secs(3600); // 1 hour expiration
                                                           // Clean old templates (e.g., older than 1 hour)
    app_state.clean_old_templates(expiration_duration).await;
}

pub async fn initialize_state() -> AppState {
    // 1. Initialize Handlebars
    let handlebars = Handlebars::new(); // Removed mut

    // Register Handlebars helpers here (if you have any)
    // handlebars.register_helper(...);

    // 2. Create the template cache
    let template_cache: Arc<RwLock<LruCache<String, CachedTemplate>>> =
        Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(100).unwrap())));

    // 3. Construct the AppState
    let app_state = AppState {
        handlebars,
        template_cache,
    };

    // 4. Spawn a background task to clean the cache periodically
    let app_state_clone = app_state.clone(); // Clone for the background task
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(600)); // Every 10 minutes (600 seconds)
        loop {
            interval.tick().await; // Wait for the next tick
            clean_template_cache(&app_state_clone).await;
        }
    });

    app_state
}

#[tokio::main]
async fn main() {
    let app_state = initialize_state().await;
    // Creates the Axum router with routes for MJML conversion, template listing, and template upload.
    let app = Router::new()
        .route("/convert", post(convert_mjml))
        .route("/templates", get(list_templates))
        .route("/templates", post(upload_template))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));

    println!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod tests;
