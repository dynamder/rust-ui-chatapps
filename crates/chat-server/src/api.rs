//! 小小的一层 HTTP：REST 负责请求/响应，SSE 负责把 [`ChatEvent`] 推给 web 系 UI。
//! 业务逻辑全部在 `chat_core` 里，这里只做编解码和错误映射。

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chat_core::{ChatService, Conversation, ConversationSummary, Message, MessageHit, MessagePage};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub service: ChatService,
}

pub fn router(service: ChatService) -> Router {
    let state = AppState { service };

    Router::new()
        .route("/api/health", get(health))
        .route(
            "/api/conversations",
            get(list_conversations).post(create_conversation),
        )
        .route(
            "/api/conversations/{id}",
            get(get_conversation)
                .patch(rename_conversation)
                .delete(delete_conversation),
        )
        .route(
            "/api/conversations/{id}/messages",
            get(list_messages).post(send_message).delete(clear_messages),
        )
        .route("/api/conversations/{id}/read", post(mark_read))
        .route("/api/search", get(search_messages))
        .route("/api/events", get(events))
        // web 系框架的开发服务器通常在不同的端口上，直接放开 CORS。
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// ---------------------------------------------------------------- health

#[derive(Debug, Serialize)]
struct Health {
    ok: bool,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}

// ---------------------------------------------------------------- 会话

#[derive(Debug, Deserialize)]
struct CreateConversationBody {
    #[serde(default)]
    title: Option<String>,
}

async fn create_conversation(
    State(state): State<AppState>,
    body: Option<Json<CreateConversationBody>>,
) -> Result<(StatusCode, Json<Conversation>), ApiError> {
    let title = body.and_then(|Json(body)| body.title);
    let conversation = state.service.create_conversation(title.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(conversation)))
}

async fn list_conversations(
    State(state): State<AppState>,
) -> Result<Json<Vec<ConversationSummary>>, ApiError> {
    Ok(Json(state.service.list_conversations().await?))
}

async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ConversationSummary>, ApiError> {
    Ok(Json(state.service.get_conversation(&id).await?))
}

#[derive(Debug, Deserialize)]
struct RenameConversationBody {
    title: String,
}

async fn rename_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RenameConversationBody>,
) -> Result<Json<Conversation>, ApiError> {
    Ok(Json(
        state.service.rename_conversation(&id, &body.title).await?,
    ))
}

async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.service.delete_conversation(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn mark_read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ConversationSummary>, ApiError> {
    Ok(Json(state.service.mark_read(&id).await?))
}

// ---------------------------------------------------------------- 消息

#[derive(Debug, Deserialize)]
struct MessagePageQuery {
    before_id: Option<i64>,
    limit: Option<i64>,
}

async fn list_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<MessagePageQuery>,
) -> Result<Json<MessagePage>, ApiError> {
    Ok(Json(
        state
            .service
            .messages(&id, query.before_id, query.limit)
            .await?,
    ))
}

#[derive(Debug, Deserialize)]
struct SendMessageBody {
    content: String,
}

async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageBody>,
) -> Result<(StatusCode, Json<Message>), ApiError> {
    let message = state.service.send_user_message(&id, &body.content).await?;
    Ok((StatusCode::CREATED, Json(message)))
}

#[derive(Debug, Serialize)]
struct ClearedMessages {
    deleted: u64,
}

async fn clear_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ClearedMessages>, ApiError> {
    let deleted = state.service.clear_messages(&id).await?;
    Ok(Json(ClearedMessages { deleted }))
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    conversation_id: Option<String>,
    limit: Option<i64>,
}

async fn search_messages(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<MessageHit>>, ApiError> {
    Ok(Json(
        state
            .service
            .search_messages(&query.q, query.conversation_id.as_deref(), query.limit)
            .await?,
    ))
}

// ---------------------------------------------------------------- 事件流

#[derive(Debug, Deserialize)]
struct EventsQuery {
    conversation_id: Option<String>,
}

/// `GET /api/events`（可选 `?conversation_id=`）：SSE 长连接。
///
/// 每个事件形如 `event: chat` + `data: {"type":"message_created",...}`。
async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let filter = query.conversation_id;

    let stream = BroadcastStream::new(state.service.subscribe()).filter_map(move |received| {
        // 每个 SSE 连接各自克隆一份过滤条件。
        let conversation_id = filter.clone();
        async move {
            let event = received.ok()?;
            if let Some(conversation_id) = &conversation_id {
                if event.conversation_id() != Some(conversation_id.as_str()) {
                    return None;
                }
            }
            let data = serde_json::to_string(&event).ok()?;
            Some(Ok(Event::default().event("chat").data(data)))
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

// ---------------------------------------------------------------- 错误映射

pub struct ApiError(chat_core::Error);

impl From<chat_core::Error> for ApiError {
    fn from(error: chat_core::Error) -> Self {
        Self(error)
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            chat_core::Error::ConversationNotFound(_) => StatusCode::NOT_FOUND,
            chat_core::Error::EmptyContent
            | chat_core::Error::EmptyTitle
            | chat_core::Error::ContentTooLong { .. }
            | chat_core::Error::TitleTooLong { .. }
            | chat_core::Error::InvalidQuery(_) => StatusCode::BAD_REQUEST,
            chat_core::Error::Db(_) | chat_core::Error::Migrate(_) | chat_core::Error::Io(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        if status.is_server_error() {
            tracing::error!(error = %self.0, "请求处理失败");
        }

        let body = ErrorResponse {
            error: ErrorDetail {
                code: self.0.code(),
                message: self.0.to_string(),
            },
        };
        (status, Json(body)).into_response()
    }
}
