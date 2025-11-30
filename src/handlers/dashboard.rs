use axum::{ extract::State, http::{ header, HeaderMap, StatusCode }, response::IntoResponse };
use std::fs;
use base64::{ engine::general_purpose, Engine as _ };

use crate::{ db::AppState, error::AppError };

pub async fn serve_dashboard(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let html_path = "views/index.html";

    let mut html_content = fs
        ::read_to_string(html_path)
        .map_err(|e| {
            tracing::error!("Failed to read dashboard HTML: {}", e);
            AppError::InternalError(anyhow::anyhow!("Failed to load dashboard"))
        })?;

    let env_mode = match state.config.server.environment {
        crate::config::Environment::Development => "Development",
        crate::config::Environment::Production => "Production",
    };

    html_content = html_content.replace("{{ENV_MODE}}", env_mode);
    html_content = html_content.replace("{{SERVER_PORT}}", &state.config.server.port.to_string());

    Ok((StatusCode::OK, [(header::CONTENT_TYPE, "text/html; charset=utf-8")], html_content))
}

pub async fn serve_docs(
    State(state): State<AppState>,
    headers: HeaderMap
) -> Result<axum::response::Response, AppError> {
    tracing::debug!("Docs auth - Expected username: {}", state.config.docs.username);
    tracing::debug!("Docs auth - Expected password: {}", state.config.docs.password);
    
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            tracing::debug!("Auth header received: {}", auth_str);
            if auth_str.starts_with("Basic ") {
                let encoded = &auth_str[6..];
                if let Ok(decoded_bytes) = general_purpose::STANDARD.decode(encoded) {
                    if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                        let parts: Vec<&str> = decoded_str.splitn(2, ':').collect();
                        tracing::debug!("Decoded credentials - username: {}, password: {}", 
                            parts.get(0).unwrap_or(&""), parts.get(1).unwrap_or(&""));
                        if
                            parts.len() == 2 &&
                            parts[0] == state.config.docs.username &&
                            parts[1] == state.config.docs.password
                        {
                            tracing::info!("Docs authentication successful");
                            let html_path = "views/docs.html";
                            let html_content = fs
                                ::read_to_string(html_path)
                                .map_err(|e| {
                                    tracing::error!("Failed to read docs HTML: {}", e);
                                    AppError::InternalError(
                                        anyhow::anyhow!("Failed to load documentation")
                                    )
                                })?;

                            let response = axum::response::Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                                .body(axum::body::Body::from(html_content))
                                .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to build response: {}", e)))?;
                            
                            return Ok(response);
                        }
                    }
                }
            }
        }
    }

    let response = axum::response::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::WWW_AUTHENTICATE, "Basic realm=\"Alimentify API Documentation\"")
        .body(axum::body::Body::from(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Authentication Required</title>
    <style>
        body { 
            font-family: 'Inter', sans-serif; 
            background: #0a0a0a; 
            color: #fff; 
            display: flex; 
            align-items: center; 
            justify-content: center; 
            height: 100vh; 
            margin: 0;
        }
        .container { 
            text-align: center; 
            background: #1a1a1a; 
            padding: 40px; 
            border-radius: 12px; 
            border: 1px solid #333;
        }
        h1 { color: #a855f7; margin-bottom: 10px; }
        p { color: #9ca3af; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🔒 Authentication Required</h1>
        <p>Please provide valid credentials to access the API documentation.</p>
    </div>
</body>
</html>
"#))
        .map_err(|e| AppError::InternalError(anyhow::anyhow!("Failed to build response: {}", e)))?;
    
    Ok(response)
}
