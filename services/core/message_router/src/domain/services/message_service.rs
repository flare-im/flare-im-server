use crate::domain::{
    entities::MessageStatus,
    repositories::{
        MessageRepository, RouteRepository, FriendRepository, GroupRepository,
        ContentFilterRepository,
    },
};
use async_trait::async_trait;
use std::sync::Arc;
use anyhow::Result;
use log::{info, error, warn};
use common::utils::msg_utils::is_group_message;
use proto_crate::api::im::common::MessageData;
use crate::domain::repositories::GroupMemberQuery;
use crate::entities::{MessageProcessResult, PreProcessCode};
use crate::services::MessageService;



pub struct MessageServiceImpl {
    message_repository: Arc<dyn MessageRepository>,
    route_repository: Arc<dyn RouteRepository>,
    friend_repository: Arc<dyn FriendRepository>,
    group_repository: Arc<dyn GroupRepository>,
    content_filter_repository: Arc<dyn ContentFilterRepository>,
}

impl MessageServiceImpl {
    pub fn new(
        message_repository: Arc<dyn MessageRepository>,
        route_repository: Arc<dyn RouteRepository>,
        friend_repository: Arc<dyn FriendRepository>,
        group_repository: Arc<dyn GroupRepository>,
        content_filter_repository: Arc<dyn ContentFilterRepository>,
    ) -> Self {
        Self {
            message_repository,
            route_repository,
            friend_repository,
            group_repository,
            content_filter_repository,
        }
    }

    /// 检查是否需要离线推送
    fn need_offline_push(&self, message: &MessageData) -> bool {
        // 检查消息配置
        message.options.get("need_offline_push")
            .map(|v| v == "true")
            .unwrap_or(true) // 默认开启离线推送
    }

    /// 检查消息是否需要重试
    fn need_retry(&self, message: &MessageData) -> bool {
        message.options.get("need_retry")
            .map(|v| v == "true")
            .unwrap_or(true) // 默认开启重试
    }

    /// 检查消息格式
    fn check_message_format(&self, message: &MessageData) -> bool {
        // 检查必要字段
        if message.server_msg_id.is_empty() || message.send_id.is_empty() {
            return false;
        }

        // 检查消息内容
        if message.content.is_empty() {
            return false;
        }

        true
    }
    async fn check_content_security(&self, message: &MessageData) -> Result<bool> {
        // 1. 检查文本内容
        let text_result = self.content_filter_repository
            .check(message)
            .await?;

        if !text_result.passed {
            warn!(
                "Message content security check failed: {:?}, reason: {:?}",
                message.server_msg_id,
                text_result.reason
            );
            return Ok(false);
        }

        Ok(true)
    }

    async fn check_permission(&self, message: &MessageData) -> Result<PreProcessCode> {
        if is_group_message(message){
            // 检查群成员权限
            let member_status = self.group_repository
                .check_member_status(&message.group_id, &message.send_id)
                .await?;

            if !member_status.is_member {
                return Ok(PreProcessCode::NotGroupMember);
            }

            if member_status.is_muted {
                return Ok(PreProcessCode::Muted);
            }

        } else {
            // 检查好友关系
            let friendship = self.friend_repository
                .check_friendship(&message.send_id, &message.recv_id)
                .await?;

            if !friendship.is_friend {
                return Ok(PreProcessCode::NotFriend);
            }

            if friendship.in_blacklist {
                return Ok(PreProcessCode::InBlacklist);
            }
        }

        Ok(PreProcessCode::Ok)
    }

    async fn check_business_rules(&self, message: &MessageData) -> Result<PreProcessCode> {
        // 1. 检查消息频率限制
        if let Some(limit) = message.options.get("rate_limit") {
            let rate_limit = limit.parse::<i32>().unwrap_or(10); // 默认每分钟10条
            let count = self.message_repository
                .get_recent_message_count(&message.send_id, 60)
                .await?;
            
            if count >= rate_limit {
                return Ok(PreProcessCode::FrequencyLimit);
            }
        }

        // 2. 检查消息大小限制
        let content_size = message.content.len();
        if content_size > 1024 * 1024 { // 1MB
            return Ok(PreProcessCode::ContentLengthLimit);
        }

        // 3. 检查附件大小
        if let Some(attachment_size) = message.options.get("attachment_size") {
            let size = attachment_size.parse::<i64>().unwrap_or(0);
            if size > 100 * 1024 * 1024 { // 100MB
                return Ok(PreProcessCode::AttachmentSizeLimit);
            }
        }

        // 4. 检查会话消息数量限制
        if is_group_message(message) {
            // 群聊消息限制
            let daily_limit = message.options.get("group_daily_limit")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(1000); // 默认群聊每天1000条

            let daily_count = self.message_repository
                .get_group_daily_message_count(&message.group_id)
                .await?;

            if daily_count >= daily_limit {
                return Ok(PreProcessCode::GroupMessageLimit);
            }

            // 检查群状态
            let group_status = self.group_repository
                .get_group_status(&message.group_id)
                .await?;
            
            if !group_status.is_active {
                return Ok(PreProcessCode::GroupDissolved);
            }

            if group_status.is_muted {
                return Ok(PreProcessCode::GroupMuted);
            }

        } else {
            // 单聊消息限制
            let daily_limit = message.options.get("private_daily_limit")
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(200); // 默认私聊每天200条

            let daily_count = self.message_repository
                .get_private_daily_message_count(&message.send_id, &message.recv_id)
                .await?;

            if daily_count >= daily_limit {
                return Ok(PreProcessCode::PrivateMessageLimit);
            }
        }

        // 5. 检查用户状态
        let user_status = self.message_repository
            .get_user_status(&message.send_id)
            .await?;

        if user_status.is_banned {
            return Ok(PreProcessCode::UserBanned);
        }

        // 6. 检查设备状态
        if let Some(device_id) = message.options.get("device_id") {
            let device_status = self.message_repository
                .get_device_status(device_id)
                .await?;

            if device_status.is_banned {
                return Ok(PreProcessCode::DeviceBanned);
            }
        }
        Ok(PreProcessCode::Ok)
    }

}

#[async_trait]
impl MessageService for MessageServiceImpl {
    async fn pre_process(&self, message: &MessageData) -> Result<PreProcessCode> {
        // 1. 基础格式校验
        if !self.check_message_format(message) {
            return Ok(PreProcessCode::InvalidFormat);
        }

        // 2. 内容安全检查
        if !self.check_content_security(message).await? {
            return Ok(PreProcessCode::InvalidContent);
        }

        // 3. 权限校验
        self.check_permission(message).await?;

        // 4. 业务规则校验
        self.check_business_rules(message).await?;

        Ok(PreProcessCode::Ok)
    }


    async fn handle_message(&self, message: &MessageData) -> Result<MessageProcessResult> {
        // 1. 获取接收方路由信息
        let routes = self.route_repository.get_routes_with_weight(&message.recv_id).await?;
        
        // 2. 处理消息推送
        let mut error_msg = None;
        if routes.is_empty() {
            // 如果用户离线，发送离线通知
            if self.need_offline_push(message) {
                self.message_repository.send_offline_notification(message.recv_id.clone().as_str(),message).await?;
                error_msg = Some("Recipient offline, message queued for offline push".to_string());
            }
        } else {
            // 如果用户在线，推送消息到网关
            self.message_repository.push_message(message, routes.clone()).await?;
        }

        // 3. 构建处理结果
        Ok(MessageProcessResult {
            message_id: message.server_msg_id.clone(),
            success: true,
            error: error_msg,
            routes: routes.into_iter().map(|r| r.address).collect(),
        })
    }

    async fn handle_group_message(&self, message: &MessageData) -> Result<MessageProcessResult> {
        const BATCH_SIZE: i32 = 1000; // 每批处理1000个成员
        
        // 1. 获取群成员总数
        let total_members = self.group_repository.get_member_count(&message.group_id).await?;
        
        let mut all_routes = Vec::new();
        let mut offline_members = Vec::new();
        let mut cursor = None;
        let mut success = false;

        // 2. 分批获取并处理群成员
        while {
            // 获取一批群成员
            let query = GroupMemberQuery {
                page_size: BATCH_SIZE,
                cursor: cursor.clone(),
                role_filter: None,
                active_only: true, // 只获取活跃成员
            };
            
            let page = self.group_repository
                .get_group_members_paged(&message.group_id, query)
                .await?;

            // 过滤掉发送者自己
            let current_batch: Vec<String> = page.members
                .into_iter()
                .filter(|id| id != &message.send_id)
                .collect();

            if !current_batch.is_empty() {
                // 批量获取路由信息
                match self.route_repository.get_routes_with_weight_batch(&current_batch).await {
                    Ok(routes_map) => {
                        for (member_id, routes) in routes_map {
                            if routes.is_empty() {
                                offline_members.push(member_id);
                            } else {
                                all_routes.extend(routes);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get routes for batch: {}", e);
                    }
                }
            }

            // 更新游标，继续处理下一批
            cursor = page.cursor;
            page.has_more
        } {}

        // 3. 批量推送在线消息
        if !all_routes.is_empty() {
            // 分批推送消息
            for routes_chunk in all_routes.chunks(BATCH_SIZE as usize) {
                match self.message_repository.push_message(message, routes_chunk.to_vec()).await {
                    Ok(_) => success = true,
                    Err(e) => {
                        error!("Failed to push message to chunk: {}", e);
                    }
                }
            }
        }

        // 4. 异步处理离线消息
        if !offline_members.is_empty() && self.need_offline_push(message) {
            let message_repo = self.message_repository.clone();
            let message = message.clone();
            let offline_members = offline_members.clone();
            
            tokio::spawn(async move {
                for member_chunk in offline_members.chunks(BATCH_SIZE as usize) {
                    for member_id in member_chunk {
                        if let Err(e) = message_repo
                            .send_offline_notification(member_id, &message)
                            .await
                        {
                            error!("Failed to send offline notification to {}: {}", member_id, e);
                        }
                    }
                }
            });
        }

        // 5. 构建处理结果
        Ok(MessageProcessResult {
            message_id: message.server_msg_id.clone(),
            success,
            error: if !success {
                Some(format!(
                    "Message processed: {} online members, {} offline members",
                    all_routes.len() / 2, // 每个成员可能有多个路由
                    offline_members.len()
                ))
            } else {
                None
            },
            routes: all_routes.into_iter().map(|r| r.address).collect(),
        })
    }

    async fn handle_message_distribution(&self, message: &MessageData) -> Result<()> {
        self.message_repository.handle_message_distribution(message).await
    }

    async fn handle_message_storage(&self, message: &MessageData) -> Result<()> {
        // 1. 消息持久化
        self.message_repository.save_message(message).await?;

        // 3. 更新会话最新消息
        if is_group_message(message) {
            self.group_repository
                .update_last_message(&message.group_id, message)
                .await?;
        }

        Ok(())
    }


    async fn handle_message_sync(&self, message: &MessageData) -> Result<()> {
        // 1. 获取用户的其他在线设备
        let other_routes = self.route_repository
            .get_routes_with_weight(&message.recv_id)
            .await?;

        // 2. 同步到其他设备
        for route in other_routes {
            if route.address != message.options.get("current_route").map(String::as_str).unwrap_or("") {
                // TODO: 发送同步消息到其他设备
                info!("Syncing message {} to device at {}", message.server_msg_id, route.address);
            }
        }

        Ok(())
    }

    /// 处理消息重试
    /// 
    /// 该方法实现了消息发送失败后的重试机制，主要功能包括：
    /// 1. 检查消息是否需要重试
    /// 2. 验证重试次数是否超过限制
    /// 3. 使用指数退避算法计算下次重试时间
    /// 4. 更新消息状态和重试相关信息
    /// 5. 保存更新后的消息
    /// 
    /// # 重试策略
    /// - 使用指数退避算法，每次重试的间隔时间会翻倍
    /// - 默认基础延迟时间为1秒
    /// - 最大重试次数默认为3次
    /// - 超过最大重试次数后，消息状态会被标记为失败
    /// 
    /// # 消息选项
    /// 该方法会更新消息的以下选项：
    /// - retry_count: 当前重试次数
    /// - next_retry_time: 下次重试时间
    /// - last_retry_time: 上次重试时间
    /// - base_delay: 基础延迟时间（可选，默认1000ms）
    /// - max_retries: 最大重试次数（可选，默认3次）
    /// 
    /// # 参数
    /// * `message` - 需要重试的消息
    /// 
    /// # 返回
    /// * `Result<(), Error>` - 处理结果
    ///   - Ok(()) - 重试处理成功
    ///   - Err(e) - 重试处理失败，包含具体错误信息
    /// 
    /// # 错误处理
    /// - 如果消息不需要重试，直接返回 Ok(())
    /// - 如果重试次数超过限制，将消息状态更新为失败
    /// - 如果保存更新失败，返回具体错误信息
    /// 
    /// # 日志记录
    /// - 记录重试次数超限的错误日志
    /// - 记录重试调度的信息日志
    /// - 记录保存失败的错误日志
    async fn handle_message_retry(&self, message: &MessageData) -> Result<()> {
        // 检查消息是否需要重试
        if !self.need_retry(message) {
            return Ok(());
        }

        // 1. 检查重试次数和最大重试次数
        let retry_count = message.options.get("retry_count")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);

        let max_retries = message.options.get("max_retries")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(3);

        // 如果超过最大重试次数，将消息标记为失败并存入死信队列
        if retry_count >= max_retries {
            error!(
                "Message {} exceeded max retry count: {}/{}",
                message.server_msg_id, retry_count, max_retries
            );
            
            // 更新消息状态为失败
            self.message_repository
                .update_message_status(&message.server_msg_id, MessageStatus::Failed)
                .await?;
            
            // 保存到死信队列
            let error_msg = format!(
                "Message processing failed after {} retries. Last retry at: {}", 
                retry_count,
                message.options.get("last_retry_time").unwrap_or(&"unknown".to_string())
            );
            
            self.message_repository
                .save_to_dead_letter(message, error_msg, retry_count)
                .await?;
            
            return Ok(());
        }

        // 2. 计算指数退避时间
        let base_delay = message.options.get("base_delay")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1000); // 默认1秒

        let delay = base_delay * (1 << retry_count);
        let next_retry_time = chrono::Utc::now().timestamp_millis() + delay;

        // 3. 更新重试状态
        let mut retry_message = message.clone();
        retry_message.status = MessageStatus::Pending as i32;
        
        // 更新消息选项，记录重试相关信息
        let mut options = retry_message.options.clone();
        options.insert("retry_count".to_string(), (retry_count + 1).to_string());
        options.insert("next_retry_time".to_string(), next_retry_time.to_string());
        options.insert("last_retry_time".to_string(), chrono::Utc::now().timestamp_millis().to_string());
        retry_message.options = options;

        // 4. 保存更新后的消息
        self.message_repository.save_message(&retry_message).await?;

        // 5. 重新分发消息
        info!(
            "Retrying message {} ({}/{})",
            message.server_msg_id,
            retry_count + 1,
            max_retries
        );

        // 根据消息类型选择不同的处理方式
       self.message_repository.handle_message_distribution(&retry_message).await?;
        Ok(())
    }
}