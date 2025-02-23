use axum::{
    extract::{Json, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use tokio::io::{AsyncWriteExt};
use tokio::io::BufWriter as AsyncBufWriter;
use tokio::fs::File;

use mrml::{prelude::render::RenderOptions, self};

use crate::app_state::AppState;
use crate::models::MjmlInput;

pub async fn convert_mjml(
    State(app_state): State<AppState>,
    Json(payload): Json<MjmlInput>,
) -> Result<Response, (StatusCode, String)> {
    let mjml_content = match &payload.template {
        Some(template_name) => {
            let template_path = format!("{}/{}", app_state.template_dir.display(), template_name);
            app_state
                .get_template(&template_path)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to read template file {}: {}", template_path, e),
                    )
                })?
        }
        None => payload
            .mjml
            .clone()
            .ok_or((StatusCode::BAD_REQUEST, "Missing MJML input".to_string()))?,
    };

    let mjml_content = app_state
        .handlebars
        .render_template(&mjml_content, &payload.payload)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Handlebars rendering error: {}", e),
            )
        })?;
    let parsed = mrml::parse(&mjml_content).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid MJML input: {}", e),
        )
    })?;
    let rendered = parsed.render(&RenderOptions::default()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Couldn't render MJML template: {}", e),
        )
    })?;
    Ok((StatusCode::OK, rendered).into_response())
}

/// Lists all MJML templates in the ./templates directory.
pub async fn list_templates(
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    use std::fs;
    use axum::Json;
    let templates_dir = app_state.template_dir.clone();
    let entries = match fs::read_dir(templates_dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
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
pub async fn upload_template(
    State(app_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    use tracing::error;
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
                        let file_path = format!("./{}/{}", app_state.clone().template_dir.display(), file_name);
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
                error!("Invalid UTF-8 sequence: {}", e);
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
