use thiserror::Error as ThisError;

/// 核心库对外暴露的错误。HTTP 层用 [`Error::code`] 映射成稳定的错误码。
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("会话不存在: {0}")]
    ConversationNotFound(String),
    #[error("消息内容不能为空")]
    EmptyContent,
    #[error("会话标题不能为空")]
    EmptyTitle,
    #[error("消息过长：最多 {max} 个字符")]
    ContentTooLong { max: usize },
    #[error("标题过长：最多 {max} 个字符")]
    TitleTooLong { max: usize },
    #[error("无效的查询参数: {0}")]
    InvalidQuery(String),
    #[error("数据库错误: {0}")]
    Db(#[from] sqlx::Error),
    #[error("数据库迁移失败: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("文件系统错误: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// 给 HTTP / 日志用的稳定错误码，不随错误文案变化。
    pub fn code(&self) -> &'static str {
        match self {
            Error::ConversationNotFound(_) => "not_found",
            Error::EmptyContent
            | Error::EmptyTitle
            | Error::ContentTooLong { .. }
            | Error::TitleTooLong { .. } => "invalid_input",
            Error::InvalidQuery(_) => "invalid_query",
            Error::Db(_) | Error::Migrate(_) | Error::Io(_) => "internal",
        }
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::ConversationNotFound(_))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
