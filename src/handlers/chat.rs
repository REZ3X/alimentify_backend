use axum::{ extract::{ Path, State }, response::{ IntoResponse, Json }, Extension };
use mongodb::bson::{ doc, oid::ObjectId };
use serde::{ Deserialize, Serialize };
use chrono::Utc;
use futures::stream::TryStreamExt;
use std::sync::Arc;

use crate::{
    db::AppState,
    error::AppError,
    models::{ Claims, ChatSession, ChatMessage, MessageRole },
    services::{ email_service::EmailService, chat_agent_service::ChatAgentService },
};

#[derive(Debug, Deserialize)]
pub struct CreateChatRequest {
    pub initial_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatSessionResponse {
    pub success: bool,
    pub session: ChatSessionDto,
}

#[derive(Debug, Serialize)]
pub struct ChatSessionDto {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i32,
}

#[derive(Debug, Serialize)]
pub struct ChatSessionsListResponse {
    pub success: bool,
    pub sessions: Vec<ChatSessionDto>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
    #[serde(default)]
    pub image_data: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub success: bool,
    pub user_message: ChatMessageDto,
    pub assistant_message: ChatMessageDto,
    pub session: ChatSessionDto,
}

#[derive(Debug, Serialize)]
pub struct ChatMessageDto {
    pub id: String,
    pub role: String,
    pub content: String,
    pub image_url: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDto>>,
    pub tool_results: Option<Vec<ToolResultDto>>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ToolCallDto {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ToolResultDto {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ChatMessagesResponse {
    pub success: bool,
    pub messages: Vec<ChatMessageDto>,
}

pub async fn create_chat_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateChatRequest>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    tracing::info!("Creating new chat session for user: {}", claims.sub);

    let gemini = state.gemini_service.clone();
    let email_service = Arc::new(
        EmailService::new(
            state.config.brevo.smtp_host.clone(),
            state.config.brevo.smtp_port,
            state.config.brevo.smtp_user.clone(),
            state.config.brevo.smtp_pass.clone(),
            state.config.brevo.from_email.clone(),
            state.config.brevo.from_name.clone()
        )
    );
    let agent = ChatAgentService::new(gemini.clone(), email_service);

    let title = if let Some(ref msg) = payload.initial_message {
        agent.generate_chat_title(msg).await.unwrap_or_else(|_| "New Chat".to_string())
    } else {
        "New Chat".to_string()
    };

    let now = Utc::now();
    let session = ChatSession {
        id: None,
        user_id,
        title,
        created_at: now,
        updated_at: now,
        message_count: 0,
    };

    let result = state.db
        .collection::<ChatSession>("chat_sessions")
        .insert_one(&session, None).await
        .map_err(|e| {
            tracing::error!("Failed to create chat session: {}", e);
            AppError::InternalError(e.into())
        })?;

    let session_id = result.inserted_id.as_object_id().unwrap();

    let response = ChatSessionDto {
        id: session_id.to_hex(),
        title: session.title,
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
        message_count: 0,
    };

    Ok(
        Json(ChatSessionResponse {
            success: true,
            session: response,
        })
    )
}

pub async fn get_chat_sessions(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let mut cursor = state.db
        .collection::<ChatSession>("chat_sessions")
        .find(
            doc! { "user_id": user_id },
            mongodb::options::FindOptions
                ::builder()
                .sort(doc! { "updated_at": -1 })
                .build()
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut sessions = Vec::new();
    while
        let Some(session) = cursor.try_next().await.map_err(|e| AppError::InternalError(e.into()))?
    {
        sessions.push(ChatSessionDto {
            id: session.id.unwrap().to_hex(),
            title: session.title,
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
            message_count: session.message_count,
        });
    }

    Ok(
        Json(ChatSessionsListResponse {
            success: true,
            sessions,
        })
    )
}

pub async fn get_chat_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let session_oid = ObjectId::parse_str(&session_id).map_err(|_|
        AppError::BadRequest("Invalid session ID".to_string())
    )?;

    let session = state.db
        .collection::<ChatSession>("chat_sessions")
        .find_one(doc! { "_id": session_oid, "user_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Chat session not found".to_string()))?;

    Ok(
        Json(ChatSessionResponse {
            success: true,
            session: ChatSessionDto {
                id: session.id.unwrap().to_hex(),
                title: session.title,
                created_at: session.created_at.to_rfc3339(),
                updated_at: session.updated_at.to_rfc3339(),
                message_count: session.message_count,
            },
        })
    )
}

pub async fn send_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<String>,
    Json(payload): Json<SendMessageRequest>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let session_oid = ObjectId::parse_str(&session_id).map_err(|_|
        AppError::BadRequest("Invalid session ID".to_string())
    )?;

    let session = state.db
        .collection::<ChatSession>("chat_sessions")
        .find_one(doc! { "_id": session_oid, "user_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Chat session not found".to_string()))?;

    tracing::info!("Sending message in session: {}", session_id);

    let mut message_content = payload.message.clone();
    let mut image_data_url: Option<String> = None;
    let gemini = state.gemini_service.clone();

    if
        let (Some(image_data), Some(mime_type)) = (
            payload.image_data.as_ref(),
            payload.mime_type.as_ref(),
        )
    {
        tracing::info!("Processing image in chat message");

        image_data_url = Some(format!("data:{};base64,{}", mime_type, image_data));

        use base64::{ engine::general_purpose, Engine as _ };
        let image_bytes = general_purpose::STANDARD
            .decode(image_data)
            .map_err(|e| AppError::BadRequest(format!("Invalid image data: {}", e)))?;

        let analysis = gemini
            .analyze_food_image(&image_bytes, mime_type).await
            .map_err(|e| AppError::InternalError(e))?;

        message_content = format!("{}\n\n[Image Analysis]\n{}", message_content, analysis);
    }

    let user_message_time = Utc::now();
    let user_message = ChatMessage {
        id: None,
        session_id: session_oid,
        user_id,
        role: MessageRole::User,
        content: message_content.clone(),
        image_url: image_data_url.clone(),
        tool_calls: None,
        tool_results: None,
        created_at: user_message_time,
    };

    let user_result = state.db
        .collection::<ChatMessage>("chat_messages")
        .insert_one(&user_message, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let user_message_id = user_result.inserted_id.as_object_id().unwrap();

    let mut cursor = state.db
        .collection::<ChatMessage>("chat_messages")
        .find(
            doc! { "session_id": session_oid },
            mongodb::options::FindOptions
                ::builder()
                .sort(doc! { "created_at": 1 })
                .limit(20) 
                .build()
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut history = Vec::new();
    while let Some(msg) = cursor.try_next().await.map_err(|e| AppError::InternalError(e.into()))? {
        history.push(msg);
    }

    let email_service = Arc::new(
        EmailService::new(
            state.config.brevo.smtp_host.clone(),
            state.config.brevo.smtp_port,
            state.config.brevo.smtp_user.clone(),
            state.config.brevo.smtp_pass.clone(),
            state.config.brevo.from_email.clone(),
            state.config.brevo.from_name.clone()
        )
    );
    let agent = ChatAgentService::new(state.gemini_service.clone(), email_service);

    let (response_text, tool_calls, tool_results) = agent
        .process_message(&state, user_id, session_oid, &message_content, history).await
        .map_err(|e| {
            tracing::error!("AI agent processing failed: {}", e);
            AppError::InternalError(e)
        })?;

    let assistant_message = ChatMessage {
        id: None,
        session_id: session_oid,
        user_id,
        role: MessageRole::Assistant,
        content: response_text.clone(),
        image_url: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls.clone())
        },
        tool_results: if tool_results.is_empty() {
            None
        } else {
            Some(tool_results.clone())
        },
        created_at: Utc::now(),
    };

    let result = state.db
        .collection::<ChatMessage>("chat_messages")
        .insert_one(&assistant_message, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let message_id = result.inserted_id.as_object_id().unwrap();

    let now = Utc::now();


    let mut update_doc =
        doc! {
        "$set": { "updated_at": mongodb::bson::DateTime::from_chrono(now) },
        "$inc": { "message_count": 2 } 
    };

    if session.title == "New Chat" && session.message_count == 0 {
        let title_text = if payload.message.len() > 50 {
            format!("{}...", &payload.message[..50])
        } else {
            payload.message.clone()
        };

        let title = title_text
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ");

        update_doc.insert(
            "$set",
            doc! { 
            "title": title.clone(),
            "updated_at": mongodb::bson::DateTime::from_chrono(now)
        }
        );
    }

    state.db
        .collection::<ChatSession>("chat_sessions")
        .update_one(doc! { "_id": session_oid }, update_doc, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let new_title = if session.title == "New Chat" && session.message_count == 0 {
        let title_text = if payload.message.len() > 50 {
            format!("{}...", &payload.message[..50])
        } else {
            payload.message.clone()
        };

        title_text
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    } else {
        session.title
    };

    let updated_session = ChatSessionDto {
        id: session.id.unwrap().to_hex(),
        title: new_title,
        created_at: session.created_at.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        message_count: session.message_count + 2,
    };

    let user_message_dto = ChatMessageDto {
        id: user_message_id.to_hex(),
        role: "user".to_string(),
        content: message_content.clone(),
        image_url: image_data_url,
        tool_calls: None,
        tool_results: None,
        created_at: user_message_time.to_rfc3339(),
    };

    let assistant_message_dto = ChatMessageDto {
        id: message_id.to_hex(),
        role: "assistant".to_string(),
        content: response_text,
        image_url: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(
                tool_calls
                    .iter()
                    .map(|tc| ToolCallDto {
                        tool_name: tc.tool_name.clone(),
                        parameters: tc.parameters.clone(),
                    })
                    .collect()
            )
        },
        tool_results: if tool_results.is_empty() {
            None
        } else {
            Some(
                tool_results
                    .iter()
                    .map(|tr| ToolResultDto {
                        tool_name: tr.tool_name.clone(),
                        result: tr.result.clone(),
                        success: tr.success,
                    })
                    .collect()
            )
        },
        created_at: assistant_message.created_at.to_rfc3339(),
    };

    Ok(
        Json(SendMessageResponse {
            success: true,
            user_message: user_message_dto,
            assistant_message: assistant_message_dto,
            session: updated_session,
        })
    )
}

pub async fn get_chat_messages(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let session_oid = ObjectId::parse_str(&session_id).map_err(|_|
        AppError::BadRequest("Invalid session ID".to_string())
    )?;

    state.db
        .collection::<ChatSession>("chat_sessions")
        .find_one(doc! { "_id": session_oid, "user_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?
        .ok_or_else(|| AppError::NotFound("Chat session not found".to_string()))?;

    let mut cursor = state.db
        .collection::<ChatMessage>("chat_messages")
        .find(
            doc! { "session_id": session_oid },
            mongodb::options::FindOptions
                ::builder()
                .sort(doc! { "created_at": 1 })
                .build()
        ).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    let mut messages = Vec::new();
    while let Some(msg) = cursor.try_next().await.map_err(|e| AppError::InternalError(e.into()))? {
        messages.push(ChatMessageDto {
            id: msg.id.unwrap().to_hex(),
            role: format!("{:?}", msg.role).to_lowercase(),
            content: msg.content,
            image_url: msg.image_url,
            tool_calls: msg.tool_calls.map(|calls| {
                calls
                    .iter()
                    .map(|tc| ToolCallDto {
                        tool_name: tc.tool_name.clone(),
                        parameters: tc.parameters.clone(),
                    })
                    .collect()
            }),
            tool_results: msg.tool_results.map(|results| {
                results
                    .iter()
                    .map(|tr| ToolResultDto {
                        tool_name: tr.tool_name.clone(),
                        result: tr.result.clone(),
                        success: tr.success,
                    })
                    .collect()
            }),
            created_at: msg.created_at.to_rfc3339(),
        });
    }

    Ok(
        Json(ChatMessagesResponse {
            success: true,
            messages,
        })
    )
}

pub async fn delete_chat_session(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(session_id): Path<String>
) -> Result<impl IntoResponse, AppError> {
    let user_id = ObjectId::parse_str(&claims.sub).map_err(|_|
        AppError::BadRequest("Invalid user ID".to_string())
    )?;

    let session_oid = ObjectId::parse_str(&session_id).map_err(|_|
        AppError::BadRequest("Invalid session ID".to_string())
    )?;

    let delete_result = state.db
        .collection::<ChatSession>("chat_sessions")
        .delete_one(doc! { "_id": session_oid, "user_id": user_id }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    if delete_result.deleted_count == 0 {
        return Err(AppError::NotFound("Chat session not found".to_string()));
    }

    state.db
        .collection::<ChatMessage>("chat_messages")
        .delete_many(doc! { "session_id": session_oid }, None).await
        .map_err(|e| AppError::InternalError(e.into()))?;

    Ok(
        Json(
            serde_json::json!({
        "success": true,
        "message": "Chat session deleted successfully"
    })
        )
    )
}
