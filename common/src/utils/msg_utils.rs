use proto_crate::api::im::common::{MessageData, SessionType};

/// 判断消息是否是群消息
pub fn is_group_message(message: &MessageData) -> bool {
    !message.group_id.is_empty() && (
        message.session_type == SessionType::NormalGroup as i32 ||
        message.session_type == SessionType::SuperGroup as i32
        || message.session_type == SessionType::WorkGroup as i32
    )
}