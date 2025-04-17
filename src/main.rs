use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, sse::Event, sse::Sse, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use futures::Stream; // Remove StreamExt as it's not used
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, env, net::SocketAddr, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::process::Command;

// Default Ollama URL, can be overridden with OLLAMA_URL environment variable
fn get_ollama_url() -> String {
    env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct Model {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsResponse {
    models: Vec<Model>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigResponse {
    ollama_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChunk {
    response: Option<String>,
    done: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileContext {
    name: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StreamWithFilesRequest {
    model: String,
    prompt: String,
    files: Vec<FileContext>,
}

#[derive(Clone)]
struct AppState {
    client: Client,
    ollama_url: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get Ollama URL from environment or use default
    let ollama_url = get_ollama_url();
    
    // Application state
    let state = Arc::new(AppState {
        client: Client::new(),
        ollama_url,
    });

    // Build our application with routes
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/config", get(get_config))
        .route("/api/models", get(get_models))
        .route("/api/generate", post(generate_response))
        .route("/api/stream/:model/:prompt", get(stream_response))
        .route("/api/stream_with_files", post(stream_with_files))
        .route("/api/check_ollama", get(check_ollama))
        .route("/api/start_ollama", get(try_start_ollama))
        .route("/api/abort_generation", post(abort_generation))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    // Get port from environment or use default
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Listening on {}", addr);
    
    // Try to open the browser
    #[cfg(target_os = "linux")]
    {
        let url = format!("http://localhost:{}", port);
        let _ = Command::new("xdg-open").arg(&url).spawn();
    }
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        ollama_url: state.ollama_url.clone(),
    })
}

async fn check_ollama(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = &state.client;
    
    match client.get(&state.ollama_url).send().await {
        Ok(_) => Ok(Json(serde_json::json!({ "status": "running" }))),
        Err(_) => Ok(Json(serde_json::json!({ "status": "not_running" }))),
    }
}

async fn try_start_ollama() -> Result<Json<serde_json::Value>, StatusCode> {
    // Try to launch Ollama (this might not work in all environments)
    let result = Command::new("ollama")
        .arg("serve")
        .spawn();
    
    match result {
        Ok(_) => Ok(Json(serde_json::json!({ "status": "started" }))),
        Err(e) => Ok(Json(serde_json::json!({ 
            "status": "failed", 
            "error": e.to_string() 
        }))),
    }
}

async fn get_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ModelsResponse>, StatusCode> {
    let client = &state.client;
    
    let response = client
        .get(format!("{}/api/tags", state.ollama_url))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Error fetching models: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let models = response
        .json::<ModelsResponse>()
        .await
        .map_err(|e| {
            tracing::error!("Error parsing models response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(models))
}

async fn generate_response(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = &state.client;
    
    let ollama_request = OllamaRequest {
        model: request.model,
        prompt: request.prompt,
        stream: false,
    };
    
    let response = client
        .post(format!("{}/api/generate", state.ollama_url))
        .json(&ollama_request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Error generating response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let response_data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| {
            tracing::error!("Error parsing generation response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(response_data))
}

// Stream response from Ollama model
async fn stream_response(
    Path((model, prompt)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let client = state.client.clone();
    let ollama_url = state.ollama_url.clone();
    
    // Decode the URL-encoded prompt
    let decoded_prompt = match urlencoding::decode(&prompt) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => {
            // If decoding fails, use the original prompt
            prompt
        }
    };
    
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    
    // Spawn a task to handle the streaming
    tokio::spawn(async move {
        let ollama_request = OllamaRequest {
            model,
            prompt: decoded_prompt,
            stream: true,
        };
        
        // Log the request size for debugging
        let request_size = ollama_request.prompt.len();
        tracing::debug!("Request prompt size: {} bytes", request_size);
        
        let res = client
            .post(format!("{}/api/generate", ollama_url))
            .json(&ollama_request)
            .send()
            .await;
        
        let mut response = match res {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Ok(Event::default().data(format!("Error: {}", e)))).await;
                let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                return;
            }
        };
        
        let mut buffer = Vec::new();
        let mut full_response = String::new();
        
        // Stream chunks of data
        while let Some(chunk) = response.chunk().await.unwrap_or(None) {
            tracing::debug!("Received raw chunk of size: {} bytes", chunk.len());
            buffer.extend_from_slice(&chunk);
            
            // Process complete lines
            let mut start = 0;
            for i in 0..buffer.len() {
                if buffer[i] == b'\n' {
                    if i > start {
                        let line = String::from_utf8_lossy(&buffer[start..i]).to_string();
                        start = i + 1;
                        
                        // Parse JSON
                        match serde_json::from_str::<OllamaChunk>(&line) {
                            Ok(chunk) => {
                                if let Some(response_text) = chunk.response {
                                    tracing::debug!("Received response token: {}", response_text);
                                    // Add to full response
                                    full_response.push_str(&response_text);
                                    
                                    // Send the full response so far, but only if it's not too long
                                    if full_response.len() < 100000 { // Limit to ~100KB per message
                                        tracing::debug!("Sending full response of length: {}", full_response.len());
                                        let _ = tx.send(Ok(Event::default().data(full_response.clone()))).await;
                                    } else {
                                        // For very long responses, just send the new part
                                        tracing::debug!("Full response too large ({}), sending partial update", full_response.len());
                                        let _ = tx.send(Ok(Event::default().data(response_text))).await;
                                    }
                                }
                                
                                if chunk.done.unwrap_or(false) {
                                    tracing::info!("Generation completed, sending DONE signal");
                                    let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                                    return;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error parsing JSON: {}\nJSON was: {}", e, line);
                                let _ = tx.send(Ok(Event::default().data(format!("Error parsing JSON: {}", e)))).await;
                            }
                        }
                    }
                }
            }
            
            // Keep remaining data
            if start < buffer.len() {
                buffer = buffer[start..].to_vec();
            } else {
                buffer.clear();
            }
        }
        
        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
    });
    
    Sse::new(ReceiverStream::new(rx))
}

// Stream response with file contexts using response streaming
async fn stream_with_files(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StreamWithFilesRequest>,
) -> impl IntoResponse {
    let client = state.client.clone();
    let ollama_url = state.ollama_url.clone();
    
    // Build a prompt with context from files
    let mut context_prompt = String::new();
    
    // Add file contents as context
    for file in &request.files {
        context_prompt.push_str(&format!("File: {}\n```\n{}\n```\n\n", file.name, file.content));
    }
    
    // Log information about the request first
    tracing::info!("Processing request with {} files, prompt size: {} bytes", 
        request.files.len(), request.prompt.len());
    
    // Then combine with user prompt
    let final_prompt = if !context_prompt.is_empty() {
        format!("I have the following files for context:\n\n{}\n\nBased on these files, {}", context_prompt, request.prompt)
    } else {
        request.prompt
    };
    
    tracing::debug!("Request prompt size with files: {} bytes", final_prompt.len());
    
    // Create a channel for streaming responses with proper type annotation
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);
    let model = request.model;
    
    // Spawn a task to handle the streaming
    tokio::spawn(async move {
        let ollama_request = OllamaRequest {
            model,
            prompt: final_prompt,
            stream: true,
        };
        
        // Make the request to Ollama
        let res = client
            .post(format!("{}/api/generate", ollama_url))
            .json(&ollama_request)
            .send()
            .await;
        
        let res = match res {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Ok(Event::default().data(format!("Error: {}", e)))).await;
                let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                return;
            }
        };
        
        // Check if the request was successful
        if !res.status().is_success() {
            let status = res.status();
            let error_text = match res.text().await {
                Ok(text) => text,
                Err(_) => "Unknown error".to_string(),
            };
            
            let _ = tx.send(Ok(Event::default().data(format!("Error: HTTP {} - {}", status, error_text)))).await;
            let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
            return;
        }
        
        let mut response = res;
        let mut buffer = Vec::new();
        let mut full_response = String::new();
        
        // Stream chunks of data
        while let Some(chunk) = response.chunk().await.unwrap_or(None) {
            tracing::debug!("Received raw chunk of size: {} bytes", chunk.len());
            buffer.extend_from_slice(&chunk);
            
            // Process complete lines
            let mut start = 0;
            for i in 0..buffer.len() {
                if buffer[i] == b'\n' {
                    if i > start {
                        let line = String::from_utf8_lossy(&buffer[start..i]).to_string();
                        start = i + 1;
                        
                        // Parse JSON
                        match serde_json::from_str::<OllamaChunk>(&line) {
                            Ok(chunk) => {
                                if let Some(response_text) = chunk.response {
                                    tracing::debug!("Received response token: {}", response_text);
                                    // Add to full response
                                    full_response.push_str(&response_text);
                                    
                                    // Send the full response so far, but only if it's not too long
                                    if full_response.len() < 100000 { // Limit to ~100KB per message
                                        tracing::debug!("Sending full response of length: {}", full_response.len());
                                        let _ = tx.send(Ok(Event::default().data(full_response.clone()))).await;
                                    } else {
                                        // For very long responses, just send the new part
                                        tracing::debug!("Full response too large ({}), sending partial update", full_response.len());
                                        let _ = tx.send(Ok(Event::default().data(response_text))).await;
                                    }
                                }
                                
                                if chunk.done.unwrap_or(false) {
                                    tracing::info!("Generation completed, sending DONE signal");
                                    let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                                    return;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error parsing JSON: {}\nJSON was: {}", e, line);
                                let _ = tx.send(Ok(Event::default().data(format!("Error parsing JSON: {}", e)))).await;
                            }
                        }
                    }
                }
            }
            
            // Keep remaining data
            if start < buffer.len() {
                buffer = buffer[start..].to_vec();
            } else {
                buffer.clear();
            }
        }
        
        // Send any remaining response and done signal
        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
    });
    
    // Return SSE stream from the receiver
    Sse::new(ReceiverStream::new(rx))
}

// Handle abort notifications from client
async fn abort_generation() -> Result<Json<serde_json::Value>, StatusCode> {
    // We don't need to do much here since the client already closed the connection
    // This endpoint mainly serves as a way to log aborts if needed
    tracing::info!("Generation aborted by client");
    Ok(Json(serde_json::json!({ "status": "aborted" })))
}
