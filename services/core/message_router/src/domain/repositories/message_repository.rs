use async_trait::async_trait;
use crate::domain::entities::{Message, MessageStatus};
use anyhow::Result;
use crate::domain::repositories::RouteInfo;
use crate::entities::{DeviceStatus, UserStatus};

/// 消息仓储接口
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// 保存消息
    /// 
    /// # 参数
    /// * `message` - 要保存的消息实体
    /// 
    /// # 返回
    /// * `Result<(), Error>` - 保存成功返回Ok(()),失败返回具体错误
    async fn save_message(&self, message: &Message) -> Result<()>;
    /// 处理消息分发
    ///
    /// 包含:
    /// - 消息分发到各个网关
    /// - 消息分发到各个设备
    async fn handle_message_distribution(&self, message: &Message) -> anyhow::Result<()>;
    /// 推送消息到网关
    ///
    async fn push_message(&self, message: &Message,routers:Vec<RouteInfo>) -> Result<()>;

    /// 发送离线通知
    ///
    async fn send_offline_notification(&self,userid :&str, message: &Message) -> Result<()>;

    /// 更新消息状态
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// * `status` - 新的消息状态
    /// 
    /// # 返回
    /// * `Result<(), Error>` - 更新成功返回Ok(()),失败返回具体错误
    async fn update_message_status(&self, message_id: &str, status: MessageStatus) -> Result<()>;

    /// 获取用户最近的消息数量
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `seconds` - 时间范围（秒）
    /// 
    /// # 返回
    /// * `Result<i32, Error>` - 消息数量
    async fn get_recent_message_count(&self, user_id: &str, seconds: i32) -> Result<i32>;

    /// 获取群聊每日消息数量
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// 
    /// # 返回
    /// * `Result<i32, Error>` - 今日消息数量
    async fn get_group_daily_message_count(&self, group_id: &str) -> Result<i32>;

    /// 获取私聊每日消息数量
    /// 
    /// # 参数
    /// * `sender_id` - 发送者ID
    /// * `receiver_id` - 接收者ID
    /// 
    /// # 返回
    /// * `Result<i32, Error>` - 今日消息数量
    async fn get_private_daily_message_count(&self, sender_id: &str, receiver_id: &str) -> Result<i32>;

    /// 获取用户状态
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<UserStatus, Error>` - 用户状态信息
    async fn get_user_status(&self, user_id: &str) -> Result<UserStatus>;

    /// 获取设备状态
    /// 
    /// # 参数
    /// * `device_id` - 设备ID
    /// 
    /// # 返回
    /// * `Result<DeviceStatus, Error>` - 设备状态信息
    async fn get_device_status(&self, device_id: &str) -> Result<DeviceStatus>;

    /// 获取消息详情
    /// 
    /// # 参数
    /// * `message_id` - 消息ID
    /// 
    /// # 返回
    /// * `Result<Option<Message>, Error>` - 消息详情，不存在返回None
    async fn get_message(&self, message_id: &str) -> Result<Option<Message>>;

    /// 批量获取消息
    /// 
    /// # 参数
    /// * `message_ids` - 消息ID列表
    /// 
    /// # 返回
    /// * `Result<Vec<Message>, Error>` - 消息列表
    async fn get_messages(&self, message_ids: &[String]) -> Result<Vec<Message>>;

    /// 获取会话最新消息
    /// 
    /// # 参数
    /// * `session_id` - 会话ID
    /// * `session_type` - 会话类型（单聊/群聊）
    /// 
    /// # 返回
    /// * `Result<Option<Message>, Error>` - 最新消息，不存在返回None
    async fn get_last_message(&self, session_id: &str, session_type: i32) -> Result<Option<Message>>;

    /// 获取未读消息数量
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `session_id` - 会话ID
    /// * `session_type` - 会话类型（单聊/群聊）
    /// 
    /// # 返回
    /// * `Result<i32, Error>` - 未读消息数量
    async fn get_unread_count(&self, user_id: &str, session_id: &str, session_type: i32) -> Result<i32>;
}

