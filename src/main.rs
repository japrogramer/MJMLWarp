use axum::{
    extract::{Json, Multipart},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use handlebars::{Handlebars, Helper, Output, RenderContext, RenderError};
use mrml::{prelude::render::RenderOptions, self}; // Corrected import for RenderOptions
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
struct MjmlInput {
    mjml: String,
    payload: Value,
    template: Option<String>,
}

/// Converts MJML input to HTML using the mrml crate and handlebars templating.
use std::fs::read_to_string;

async fn convert_mjml(Json(payload): Json<MjmlInput>) -> Response {
    let template_content = match &payload.template {
        Some(template_name) => match read_to_string(format!("./templates/{}", template_name)) {
            Ok(content) => Some(content),
            Err(_) => None,
        },
        None => None,
    };

    let mjml_content = match template_content {
        Some(content) => content,
        None => payload.mjml.clone(),
    };

    let parsed = mrml::parse(&mjml_content);

    match parsed {
        Ok(root) => {

            match root.render(&RenderOptions::default()) {
                Ok(html) => (axum::http::StatusCode::OK, html).into_response(),
                Err(e) => (axum::http::StatusCode::BAD_REQUEST, format!("Couldn't render MJML template: {}", e)).into_response(),
            }
        }
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, format!("Invalid MJML input: {}", e)).into_response(),
    }
}

/// Lists all MJML templates in the ./templates directory.
async fn list_templates() -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let templates_dir = Path::new("./templates");
    let entries = match fs::read_dir(templates_dir) {
        Ok(entries) => entries,
        Err(e) => return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read templates directory: {}", e))),
    };

    let templates: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().file_name().unwrap().to_str().unwrap().to_string())
        .collect();

    Ok(Json(templates))
}

/// Uploads a new MJML template to the ./templates directory. Validates file type and MJML syntax.
async fn upload_template(mut multipart: Multipart) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let mut templates = Vec::new();
    while let Ok(Some(mut field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap().to_string();
        let content_type = field.content_type().unwrap_or("text/plain").to_string();

        if !content_type.contains("text/plain") {
            return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid file type. Only text/plain is allowed.".to_string()));
        }

        let mut buffer = Vec::new();
        while let Ok(Some(chunk)) = field.chunk().await {
            buffer.extend_from_slice(&chunk);
        }

        let mjml_content = String::from_utf8_lossy(&buffer).to_string();
        let parsed = mrml::parse(&mjml_content);

        match parsed {
            Ok(_) => {
                let file_path = format!("./templates/{}", file_name);
                let mut file = File::create(file_path).await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create file: {}", e)))?;
                file.write_all(&buffer).await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write file: {}", e)))?;
                templates.push(file_name);
            }
            Err(_) => return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid MJML input".to_string())),
        }
    }
    if templates.is_empty() {
        Ok((axum::http::StatusCode::OK, "No files uploaded".to_string()))
    } else {
        Ok((axum::http::StatusCode::OK, format!("Templates {:?} uploaded successfully", templates)))
    }
}


#[tokio::main]
async fn main() {
    // Creates the Axum router with routes for MJML conversion, template listing, and template upload.
    let app = Router::new()
        .route("/convert", post(convert_mjml))
        .route("/templates", get(list_templates))
        .route("/templates", post(upload_template));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));

    println!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
