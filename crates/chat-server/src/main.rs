use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use chat_core::{ChatConfig, ChatService};
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "chat-server",
    version,
    about = "chatapp 本地后端：REST + SSE，SQLite 存储"
)]
struct Cli {
    /// SQLite 数据库文件路径，不存在会自动创建
    #[arg(long, default_value = "data/chat.db")]
    db: PathBuf,

    /// 监听地址
    #[arg(long, default_value = "127.0.0.1:8787")]
    addr: String,

    /// 数据库为空时写入演示会话
    #[arg(long)]
    seed: bool,

    /// 关闭自动回复机器人
    #[arg(long)]
    no_auto_reply: bool,

    /// 自动回复最短延迟（毫秒）
    #[arg(long, default_value_t = 600)]
    reply_min_ms: u64,

    /// 自动回复最长延迟（毫秒）
    #[arg(long, default_value_t = 1400)]
    reply_max_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing();

    let mut config = ChatConfig::default();
    config.auto_reply.enabled = !cli.no_auto_reply;
    config.auto_reply.min_delay = Duration::from_millis(cli.reply_min_ms);
    config.auto_reply.max_delay = Duration::from_millis(cli.reply_max_ms.max(cli.reply_min_ms));
    let auto_reply_enabled = config.auto_reply.enabled;

    let service = ChatService::open_with_config(&cli.db, config)
        .await
        .with_context(|| format!("打开数据库失败: {}", cli.db.display()))?;

    if cli.seed {
        let seeded = service.seed_demo().await?;
        if seeded > 0 {
            tracing::info!(count = seeded, "已写入演示会话");
        } else {
            tracing::info!("数据库里已有会话，跳过 seed");
        }
    }

    let listener = tokio::net::TcpListener::bind(&cli.addr)
        .await
        .with_context(|| format!("无法监听 {}", cli.addr))?;
    let address = listener.local_addr()?;

    println!("chat-server 已启动");
    println!("  地址    : http://{address}");
    println!("  数据库  : {}", cli.db.display());
    println!("  事件流  : http://{address}/api/events");
    println!(
        "  自动回复: {}",
        if auto_reply_enabled { "开" } else { "关" }
    );

    axum::serve(listener, chat_server::router(service))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("chat_server=info,chat_core=info,tower_http=warn"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!(%error, "监听 Ctrl-C 失败");
        return;
    }
    println!("\n收到 Ctrl-C，正在退出…");
}
