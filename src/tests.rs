use super::*;
use axum::{Json, extract::State, response::{Response, IntoResponse}};
use serde_json::json;
use hyper::{StatusCode};
use handlebars::Handlebars;

use crate::{convert_mjml, list_templates};
use crate::MjmlInput;
use crate::AppState;

#[tokio::test]
async fn test_convert_mjml_with_template() -> Result<(), Box<dyn std::error::Error>> {
    let handlebars = Handlebars::new();
    let template_cache: Arc<RwLock<HashMap<String, CachedTemplate>>> = Arc::new(RwLock::new(HashMap::new()));
    let _template_content = handlebars.render_template(
        r#"<h1>Hello, {{name}}!</h1>"#,
        &json!({"name": "World"}),
    )?;
    let mjml_input = MjmlInput {
        mjml: None,
        payload: json!({"name": "World"}),
        template: Some("test.mjml".to_string()),
    };

    let response = convert_mjml(State(AppState { handlebars, template_cache }), Json(mjml_input)).await;
    assert_eq!(response.unwrap().status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_convert_mjml_with_mjml() -> Result<(), Box<dyn std::error::Error>> {
    let handlebars = Handlebars::new();
    let template_cache: Arc<RwLock<HashMap<String, CachedTemplate>>> = Arc::new(RwLock::new(HashMap::new()));
    let mjml_input = MjmlInput {
        mjml: Some(r#"<mjml><mj-body><mj-text>Hello, World!</mj-text></mj-body></mjml>"#.to_string()),
        payload: json!({}),
        template: None,
    };

    let response = convert_mjml(State(AppState { handlebars, template_cache }), Json(mjml_input)).await;
    assert_eq!(response.unwrap().status(), StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn test_list_templates() -> Result<(), Box<dyn std::error::Error>> {
    let response_result = list_templates().await;
    let response: Response = response_result.unwrap().into_response();
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}
