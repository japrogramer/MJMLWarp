use super::*;
use std::path::PathBuf;
use axum::{Json, extract::State, response::{Response, IntoResponse}};
use serde_json::json;
use hyper::{StatusCode};
use handlebars::Handlebars;

use crate::{convert_mjml, list_templates};
use crate::models::MjmlInput;
use crate::app_state::AppState;

#[tokio::test]
async fn test_convert_mjml_with_template() -> Result<(), Box<dyn std::error::Error>> {
    let template_dir = PathBuf::from("templates"); // Create a 'templates' directory in your project

    let mjml_input = MjmlInput {
        mjml: None,
        payload: json!({"name": "World"}),
        template: Some("test.mjml".to_string()),
    };

    let response = convert_mjml(State(AppState::new(100, template_dir)), Json(mjml_input)).await;
    assert_eq!(response.unwrap().status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_convert_mjml_with_mjml() -> Result<(), Box<dyn std::error::Error>> {
    let template_dir = PathBuf::from("templates"); // Create a 'templates' directory in your project
                                                   //
    let mjml_input = MjmlInput {
        mjml: Some(r#"<mjml><mj-body><mj-text>Hello, World!</mj-text></mj-body></mjml>"#.to_string()),
        payload: json!({}),
        template: None,
    };

    let response = convert_mjml(State(AppState::new(100, template_dir)), Json(mjml_input)).await;
    assert_eq!(response.unwrap().status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_list_templates() -> Result<(), Box<dyn std::error::Error>> {
    let template_dir = PathBuf::from("templates"); // Create a 'templates' directory in your project
    let response_result = list_templates(State(AppState::new(100, template_dir))).await;
    let response: Response = response_result.unwrap().into_response();
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}
