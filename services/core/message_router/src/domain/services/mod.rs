use async_trait::async_trait;
use proto_crate::api::im::common::MessageData;
use crate::entities::{MessageProcessResult, PreProcessCode};

mod message_service;
pub use message_service::MessageServiceImpl;

/// 消息服务接口
#[async_trait]
pub trait MessageService: Send + Sync {
    /// 消息预处理和校验
    ///
    /// 包含:
    /// - 基础格式校验
    /// - 敏感词过滤
    /// - 权限校验
    /// - 业务规则校验
    ///
    /// 返回:
    /// - PreProcessCode::Ok(0): 校验通过
    /// - PreProcessCode::InvalidFormat(1): 消息格式错误
    /// - PreProcessCode::InvalidContent(2): 内容违规
    /// - PreProcessCode::NoPermission(3): 权限不足
    /// - PreProcessCode::BusinessLimit(4): 业务规则限制
    /// - PreProcessCode::SystemError(5): 系统错误
    async fn pre_process(&self, message: &MessageData) -> anyhow::Result<PreProcessCode>;


    /// 处理单聊消息路由
    ///
    /// 包含:
    /// - 获取接收方路由信息
    /// - 选择最佳路由
    /// - 消息分发
    async fn handle_message(&self, message: &MessageData) -> anyhow::Result<MessageProcessResult>;

    /// 处理群聊消息路由
    ///
    /// 包含:
    /// - 获取群成员列表
    /// - 批量获取路由信息
    /// - 消息分发
    async fn handle_group_message(&self, message: &MessageData) -> anyhow::Result<MessageProcessResult>;

    /// 处理消息分发
    ///
    /// 包含:
    /// - 消息分发到各个网关
    /// - 消息分发到各个设备
    async fn handle_message_distribution(&self, message: &MessageData) -> anyhow::Result<()>;

    /// 处理消息存储
    ///
    /// 包含:
    /// - 消息持久化
    /// - 更新消息状态
    /// - 构建消息索引
    async fn handle_message_storage(&self, message: &MessageData) -> anyhow::Result<()>;

    /// 处理消息同步
    ///
    /// 包含:
    /// - 消息状态同步
    /// - 多端消息同步
    /// - 已读状态同步
    async fn handle_message_sync(&self, message: &MessageData) -> anyhow::Result<()>;

    /// 处理消息重试
    ///
    /// 包含:
    /// - 重试策略控制
    /// - 更新重试状态
    /// - 触发重试流程
    async fn handle_message_retry(&self, message: &MessageData) -> anyhow::Result<()>;
}
