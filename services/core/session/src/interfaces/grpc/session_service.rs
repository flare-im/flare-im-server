use tonic::{Request, Response, Status};
use crate::{
    application::session_manager::SessionManager,
    domain::{
        entities::session::{Session, SessionMember, DeviceInfo, OnlineStatus, Platform},
        repositories::session_repository::SessionRepository,
        services::session_service::SessionService,
    },
};
use api::im::service::session::{
    session_server::Session as GrpcSession,
    CreateSessionRequest,
    CreateSessionResponse,
    ConnectRequest,
    ConnectResponse,
    DisconnectRequest,
    DisconnectResponse,
    HeartbeatRequest,
    HeartbeatResponse,
    SyncSessionRequest,
    SyncSessionResponse,
    RecoverSessionRequest,
    RecoverSessionResponse,
};

pub struct SessionGrpcService<R: SessionRepository, S: SessionService> {
    session_manager: SessionManager<R, S>,
}

impl<R: SessionRepository, S: SessionService> SessionGrpcService<R, S> {
    pub fn new(session_manager: SessionManager<R, S>) -> Self {
        Self { session_manager }
    }

    // 转换设备平台
    fn convert_platform(platform: i32) -> Platform {
        match platform {
            0 => Platform::IOS,
            1 => Platform::Android,
            2 => Platform::Web,
            _ => Platform::Desktop,
        }
    }
}

#[tonic::async_trait]
impl<R: SessionRepository + Send + Sync + 'static, S: SessionService + Send + Sync + 'static> 
    GrpcSession for SessionGrpcService<R, S> 
{
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        let req = request.into_inner();
        
        let session = Session {
            id: req.session_id,
            session_type: match req.session_type {
                0 => crate::domain::entities::session::SessionType::Private,
                1 => crate::domain::entities::session::SessionType::Group,
                _ => crate::domain::entities::session::SessionType::System,
            },
            name: req.name,
            avatar_url: if req.avatar_url.is_empty() { None } else { Some(req.avatar_url) },
            members: req.members.into_iter().map(|m| SessionMember {
                user_id: m.user_id,
                role: match m.role {
                    0 => crate::domain::entities::session::MemberRole::Owner,
                    1 => crate::domain::entities::session::MemberRole::Admin,
                    _ => crate::domain::entities::session::MemberRole::Member,
                },
                joined_at: chrono::Utc::now(),
                last_active_at: chrono::Utc::now(),
                device_info: DeviceInfo {
                    device_id: String::new(),
                    platform: Platform::Web,
                    gateway_id: String::new(),
                    connection_id: String::new(),
                    connected_at: chrono::Utc::now(),
                },
                online_status: OnlineStatus::Offline,
            }).collect(),
            latest_message: None,
            unread_count: 0,
            settings: crate::domain::entities::session::SessionSettings {
                mute_notification: false,
                stick_on_top: false,
                encryption_enabled: false,
                auto_delete_after: None,
                custom_settings: std::collections::HashMap::new(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created_session = self.session_manager.create_session(session)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateSessionResponse {
            session_id: created_session.id,
            error: None,
        }))
    }

    async fn connect(
        &self,
        request: Request<ConnectRequest>,
    ) -> Result<Response<ConnectResponse>, Status> {
        let req = request.into_inner();
        
        let device_info = DeviceInfo {
            device_id: req.device_id,
            platform: Self::convert_platform(req.platform),
            gateway_id: req.gateway_id,
            connection_id: req.connection_id,
            connected_at: chrono::Utc::now(),
        };

        self.session_manager.handle_connection(&req.session_id, &req.user_id, device_info)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ConnectResponse {
            success: true,
            error: None,
        }))
    }

    async fn disconnect(
        &self,
        request: Request<DisconnectRequest>,
    ) -> Result<Response<DisconnectResponse>, Status> {
        let req = request.into_inner();

        self.session_manager.handle_disconnection(&req.session_id, &req.user_id, &req.device_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DisconnectResponse {
            success: true,
            error: None,
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();

        self.session_manager.handle_heartbeat(&req.session_id, &req.user_id, &req.device_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(HeartbeatResponse {
            success: true,
            error: None,
        }))
    }

    async fn sync_session(
        &self,
        request: Request<SyncSessionRequest>,
    ) -> Result<Response<SyncSessionResponse>, Status> {
        let req = request.into_inner();

        let state = self.session_manager.sync_session(&req.session_id, &req.user_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(SyncSessionResponse {
            session: Some(api::im::service::session::Session {
                session_id: state.session.id,
                name: state.session.name,
                avatar_url: state.session.avatar_url.unwrap_or_default(),
                session_type: match state.session.session_type {
                    crate::domain::entities::session::SessionType::Private => 0,
                    crate::domain::entities::session::SessionType::Group => 1,
                    crate::domain::entities::session::SessionType::System => 2,
                },
                unread_count: state.session.unread_count,
                last_message: state.session.latest_message.map(|m| api::im::service::session::LatestMessage {
                    message_id: m.message_id.to_string(),
                    sender_id: m.sender_id,
                    content_type: m.content_type,
                    content_preview: m.content_preview,
                    sent_at: m.sent_at.timestamp_millis(),
                }),
                online_members: state.online_members.into_iter().map(|m| api::im::service::session::SessionMember {
                    user_id: m.user_id,
                    role: match m.role {
                        crate::domain::entities::session::MemberRole::Owner => 0,
                        crate::domain::entities::session::MemberRole::Admin => 1,
                        crate::domain::entities::session::MemberRole::Member => 2,
                    },
                    device_info: Some(api::im::service::session::DeviceInfo {
                        device_id: m.device_info.device_id,
                        platform: match m.device_info.platform {
                            Platform::IOS => 0,
                            Platform::Android => 1,
                            Platform::Web => 2,
                            Platform::Desktop => 3,
                        },
                        gateway_id: m.device_info.gateway_id,
                        connection_id: m.device_info.connection_id,
                    }),
                }).collect(),
            }),
            last_sync_time: state.last_sync_time,
            error: None,
        }))
    }

    async fn recover_session(
        &self,
        request: Request<RecoverSessionRequest>,
    ) -> Result<Response<RecoverSessionResponse>, Status> {
        let req = request.into_inner();

        let device_info = DeviceInfo {
            device_id: req.device_id,
            platform: Self::convert_platform(req.platform),
            gateway_id: req.gateway_id,
            connection_id: req.connection_id,
            connected_at: chrono::Utc::now(),
        };

        let sessions = self.session_manager.recover_user_sessions(&req.user_id, device_info)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RecoverSessionResponse {
            sessions: sessions.into_iter().map(|s| api::im::service::session::Session {
                session_id: s.id,
                name: s.name,
                avatar_url: s.avatar_url.unwrap_or_default(),
                session_type: match s.session_type {
                    crate::domain::entities::session::SessionType::Private => 0,
                    crate::domain::entities::session::SessionType::Group => 1,
                    crate::domain::entities::session::SessionType::System => 2,
                },
                unread_count: s.unread_count,
                last_message: s.latest_message.map(|m| api::im::service::session::LatestMessage {
                    message_id: m.message_id.to_string(),
                    sender_id: m.sender_id,
                    content_type: m.content_type,
                    content_preview: m.content_preview,
                    sent_at: m.sent_at.timestamp_millis(),
                }),
                online_members: s.members.into_iter()
                    .filter(|m| matches!(m.online_status, OnlineStatus::Online))
                    .map(|m| api::im::service::session::SessionMember {
                        user_id: m.user_id,
                        role: match m.role {
                            crate::domain::entities::session::MemberRole::Owner => 0,
                            crate::domain::entities::session::MemberRole::Admin => 1,
                            crate::domain::entities::session::MemberRole::Member => 2,
                        },
                        device_info: Some(api::im::service::session::DeviceInfo {
                            device_id: m.device_info.device_id,
                            platform: match m.device_info.platform {
                                Platform::IOS => 0,
                                Platform::Android => 1,
                                Platform::Web => 2,
                                Platform::Desktop => 3,
                            },
                            gateway_id: m.device_info.gateway_id,
                            connection_id: m.device_info.connection_id,
                        }),
                    }).collect(),
            }).collect(),
            error: None,
        }))
    }
} 