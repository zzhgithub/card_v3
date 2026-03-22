use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchMessage {
    MatchRequest { player_name: String, version: String },
    MatchFound {
        opponent_name: String,
        host_addr: String,
        is_host: bool,
    },
    MatchCancel,
    VersionMismatch { required: String },
    Error { message: String },
    WaitingForMatch,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub opponent_name: String,
    pub host_addr: String,
    pub is_host: bool,
}

pub async fn find_match(server_url: &str, player_name: &str, version: &str) -> Result<MatchResult> {
    let (mut ws, _) = connect_async(server_url)
        .await
        .with_context(|| format!("无法连接匹配服务器: {server_url}"))?;

    let req = MatchMessage::MatchRequest {
        player_name: player_name.to_string(),
        version: version.to_string(),
    };
    let payload = serde_json::to_string(&req).context("序列化匹配请求失败")?;
    ws.send(Message::Text(payload.into()))
        .await
        .context("发送匹配请求失败")?;

    while let Some(incoming) = ws.next().await {
        let msg = incoming.context("读取匹配服务器消息失败")?;
        match msg {
            Message::Text(text) => {
                let parsed: MatchMessage =
                    serde_json::from_str(text.as_ref()).context("解析匹配服务器消息失败")?;
                match parsed {
                    MatchMessage::MatchFound {
                        opponent_name,
                        host_addr,
                        is_host,
                    } => {
                        return Ok(MatchResult {
                            opponent_name,
                            host_addr,
                            is_host,
                        });
                    }
                    MatchMessage::WaitingForMatch => continue,
                    MatchMessage::VersionMismatch { required } => {
                        bail!("客户端版本不匹配，服务器要求版本: {required}")
                    }
                    MatchMessage::Error { message } => bail!("匹配失败: {message}"),
                    MatchMessage::MatchCancel => bail!("匹配已取消"),
                    MatchMessage::MatchRequest { .. } => {
                        bail!("匹配服务器返回了无效消息类型")
                    }
                }
            }
            Message::Close(_) => bail!("匹配服务器已断开连接"),
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => {
                continue;
            }
        }
    }

    bail!("匹配服务器连接已关闭")
}
