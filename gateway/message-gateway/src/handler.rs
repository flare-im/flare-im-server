use std::collections::HashMap;
use anyhow::Result;
use tokio::sync::mpsc;
use proto_crate::message_gateway::SendMessageRequest;

#[derive(Debug)]
pub struct ConnectionHandler {
    connections: HashMap<String, Connection>,
}

#[derive(Debug)]
struct Connection {
    user_id: String,
    device_id: String,
    tx: mpsc::Sender<SendMessageRequest>,
}

impl ConnectionHandler {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub async fn handle_connect(&mut self, user_id: String, device_id: String) -> Result<()> {
        let session_id = format!("{}:{}", user_id, device_id);
        
        // 创建消息通道
        let (tx, mut rx) = mpsc::channel(100);
        
        // 存储连接信息
        self.connections.insert(session_id.clone(), Connection {
            user_id: user_id.clone(),
            device_id: device_id.clone(),
            tx,
        });

        // 启动消息处理任务
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                // 处理消息
                if let Err(e) = handle_message_delivery(msg).await {
                    log::error!("Failed to deliver message: {}", e);
                }
            }
        });

        Ok(())
    }

    pub async fn handle_disconnect(&mut self, session_id: &str) -> Result<()> {
        self.connections.remove(session_id);
        Ok(())
    }

    pub async fn handle_message(&self, message: SendMessageRequest) -> Result<()> {
        // 查找目标用户的所有设备连接
        let target_connections: Vec<_> = self.connections.values()
            .filter(|conn| conn.user_id == message.to_user_id)
            .collect();

        // 向所有设备发送消息
        for conn in target_connections {
            if let Err(e) = conn.tx.send(message.clone()).await {
                log::error!("Failed to send message to {}: {}", conn.device_id, e);
            }
        }

        Ok(())
    }
}

async fn handle_message_delivery(message: SendMessageRequest) -> Result<()> {
    // TODO: 实现实际的消息投递逻辑
    // 1. 消息持久化
    // 2. 消息路由
    // 3. 离线消息处理
    Ok(())
} 