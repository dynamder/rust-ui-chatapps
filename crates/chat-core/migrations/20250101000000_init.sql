-- 会话与消息。时间统一存 Unix 毫秒（i64），格式化交给 UI。
CREATE TABLE IF NOT EXISTS conversations (
    id                   TEXT PRIMARY KEY NOT NULL,
    title                TEXT NOT NULL,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,
    -- 列表排序用；空会话为 NULL
    last_message_at      INTEGER,
    -- 已读水位：id <= 它的 peer 消息算已读
    last_read_message_id INTEGER
);

CREATE TABLE IF NOT EXISTS messages (
    -- 自增整数 id：天然有序，可直接当分页游标
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id TEXT NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    sender          TEXT NOT NULL CHECK (sender IN ('user', 'peer')),
    content         TEXT NOT NULL,
    created_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation
    ON messages (conversation_id, id DESC);
