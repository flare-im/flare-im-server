use anyhow::Result;
use proto_crate::api::im::gateway::{
    PushMessageRequest, PushMessageResponse,
    BatchPushMessageRequest, BatchPushMessageResponse,
    BroadcastMessageRequest, BroadcastMessageResponse,
    GetUserStatusRequest, GetUserStatusResponse,
};
use crate::domain::message::MessageManager;

/// 消息服务
/// 处理消息的接收和推送
pub struct MessageService {
    message_manager: MessageManager,
}

impl MessageService {
    pub fn new() -> Self {
        Self {
            message_manager: MessageManager::new(),
        }
    }

    // ===== 消息接收相关方法 =====

    /// 处理接收到的消息
    pub async fn handle_message(&self, data: &[u8]) -> Result<Vec<u8>> {
        self.message_manager.handle_message(data).await
    }

    /// 拉取消息
    pub async fn pull_messages(&self, user_id: &str) -> Result<Vec<u8>> {
        self.message_manager.pull_messages(user_id).await
    }

    /// 处理消息确认
    pub async fn handle_ack(&self, ack_data: &[u8]) -> Result<()> {
        self.message_manager.handle_ack(ack_data).await
    }

    /// 处理消息撤回
    pub async fn handle_recall(&self, message_id: i64, user_id: &str) -> Result<()> {
        self.message_manager.handle_recall(message_id, user_id).await
    }

    /// 处理消息编辑
    pub async fn handle_edit(&self, message_id: i64, user_id: &str, new_content: &str) -> Result<()> {
        self.message_manager.handle_edit(message_id, user_id, new_content).await
    }

    /// 处理消息已读
    pub async fn handle_read(&self, user_id: &str, conversation_id: &str, message_id: i64) -> Result<()> {
        self.message_manager.handle_read(user_id, conversation_id, message_id).await
    }

    // ===== 消息推送相关方法 =====

    /// 推送消息到指定用户
    pub async fn push_message(&self, request: PushMessageRequest) -> Result<PushMessageResponse> {
        self.message_manager.push_message(request).await
    }

    /// 批量推送消息
    pub async fn batch_push_message(&self, request: BatchPushMessageRequest) -> Result<BatchPushMessageResponse> {
        self.message_manager.batch_push_message(request).await
    }

    /// 广播消息
    pub async fn broadcast_message(&self, request: BroadcastMessageRequest) -> Result<BroadcastMessageResponse> {
        self.message_manager.broadcast_message(request).await
    }

    /// 获取用户在线状态
    pub async fn get_user_status(&self, request: GetUserStatusRequest) -> Result<GetUserStatusResponse> {
        self.message_manager.get_user_status(request).await
    }

    // ===== 消息处理辅助方法 =====

    /// 处理通用请求
    pub async fn handle_request(&self, request_data: &[u8]) -> Result<Vec<u8>> {
        self.message_manager.handle_request(request_data).await
    }
} 