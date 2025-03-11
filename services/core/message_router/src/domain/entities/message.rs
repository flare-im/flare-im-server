use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Single,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub send_id: String,          // 发送者ID
    pub recv_id: String,          // 接收者ID
    pub content: Vec<u8>,         // 消息内容
    pub send_time: u64,           // 发送时间
    pub group_id: String,         // 群组ID
    pub client_msg_id: String,       // 客户端消息ID
    pub server_msg_id: String,       // 服务端消息ID
    pub send_platform_id: i32,    // 发送者平台ID
    pub send_nickname: String,    // 发送者昵称
    pub send_face_url: String,    // 发送者头像
    pub session_type: SessionType,// 会话类型
    pub msg_from: i32,            // 消息来源
    pub content_type: i32,        // 消息内容类型
    pub sequence_id: i64,         // 消息序列号
    pub create_time: u64,         // 创建时间
    pub status: i32,              // 消息状态
    pub options: HashMap<String, String>, // 消息选项
    pub offline_push_info: Option<OfflinePushInfo>, // 离线推送信息
    pub at_user_list: Vec<String>, // @用户列表
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflinePushInfo {
    pub title: String,            // 推送标题
    pub desc: String,             // 推送描述
    pub ios_push_sound: String,   // iOS推送声音
    pub ios_badge_count: bool,    // iOS角标计数
    pub signal_info: String,      // 信号信息
}

impl Message {
    pub fn is_group_message(&self) -> bool {
        matches!(self.session_type, SessionType::Group)
    }

    pub fn needs_reliability(&self) -> bool {
        self.options.get("need_receipt").copied().unwrap_or(false)
    }

    pub fn should_retry(&self) -> bool {
        let max_retry = self.options.get("max_retry_count")
            .and_then(|&b| if b { Some(3) } else { None })
            .unwrap_or(3);
        self.status == 0 && max_retry > 0
    }

    pub fn from_proto(message_data: &proto_crate::api::im::common::MessageData) -> Self {
        Self {
            send_id: message_data.send_id.clone(),
            recv_id: message_data.recv_id.clone(),
            content: message_data.content.clone(),
            send_time: message_data.send_time,
            group_id: message_data.group_id.clone(),
            client_msg_id: message_data.client_msg_id.clone(),
            server_msg_id: Uuid::new_v4().to_string(),
            send_platform_id: message_data.send_platform_id,
            send_nickname: message_data.send_nickname.clone(),
            send_face_url: message_data.send_face_url.clone(),
            session_type: if message_data.group_id.is_empty() {
                SessionType::Single
            } else {
                SessionType::Group
            },
            msg_from: message_data.msg_from,
            content_type: message_data.content_type,
            sequence_id: message_data.seq,
            create_time: message_data.create_time,
            status: message_data.status,
            options: message_data.options.clone(),
            offline_push_info: message_data.offline_push_info.as_ref().map(|info| OfflinePushInfo {
                title: info.title.clone(),
                desc: info.desc.clone(),
                ios_push_sound: info.ios_push_sound.clone(),
                ios_badge_count: info.ios_badge_count,
                signal_info: info.signal_info.clone(),
            }),
            at_user_list: message_data.at_user_list.clone(),
        }
    }

    pub fn to_proto(&self) -> proto_crate::api::im::common::MessageData {
        proto_crate::api::im::common::MessageData {
            send_id: self.send_id.clone(),
            recv_id: self.recv_id.clone(),
            content: self.content.clone(),
            send_time: self.send_time,
            group_id: self.group_id.clone(),
            client_msg_id: self.client_msg_id.clone(),
            server_msg_id: self.server_msg_id.clone(),
            send_platform_id: self.send_platform_id,
            send_nickname: self.send_nickname.clone(),
            send_face_url: self.send_face_url.clone(),
            session_type: match self.session_type {
                SessionType::Single => 1,
                SessionType::Group => 2,
            },
            msg_from: self.msg_from,
            content_type: self.content_type,
            seq: self.sequence_id,
            create_time: self.create_time,
            status: self.status,
            options: self.options.clone(),
            offline_push_info: self.offline_push_info.as_ref().map(|info| {
                proto_crate::api::im::common::OfflinePushInfo {
                    title: info.title.clone(),
                    desc: info.desc.clone(),
                    ios_push_sound: info.ios_push_sound.clone(),
                    ios_badge_count: info.ios_badge_count,
                    signal_info: info.signal_info.clone(),
                }
            }),
            at_user_list: self.at_user_list.clone(),
        }
    }
} 