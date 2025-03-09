use anyhow::Result;
use async_trait::async_trait;
use flare_core::context::AppContext;
use flare_core::flare_net::net::{Response, ResCode};
use flare_im_core::server::auth_handler::AuthHandler;
use log::{info, error};

use crate::application::auth::AuthService;

pub struct CustomAuthHandler {
    auth_service: AuthService,
}

impl CustomAuthHandler {
    pub fn new(auth_service: AuthService) -> Self {
        Self { auth_service }
    }
}

#[async_trait]
impl AuthHandler for CustomAuthHandler {
    async fn handle_login(&self, ctx: &AppContext) -> flare_core::error::Result<Response> {
        todo!()
    }

    async fn handle_logout(&self, ctx: &AppContext) -> flare_core::error::Result<Response> {
        todo!()
    }
}