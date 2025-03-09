use anyhow::Result;
use async_trait::async_trait;
use flare_core::context::AppContext;
use flare_core::flare_net::net::{Response, ResCode};
use flare_im_core::server::server::ConnectionInfo;
use flare_im_core::server::sys_handler::SystemHandler;
use log::{info, error};

use crate::application::system::SystemService;

pub struct CustomSystemHandler {
    system_service: SystemService,
}

impl CustomSystemHandler {
    pub fn new(system_service: SystemService) -> Self {
        Self { system_service }
    }
}

#[async_trait]
impl SystemHandler for CustomSystemHandler {

    async fn handle_new_connection(&self, ctx: &AppContext, conn: &ConnectionInfo) -> flare_core::error::Result<Response> {
        todo!()
    }

    async fn handle_set_background(&self, ctx: &AppContext, background: bool) -> flare_core::error::Result<Response> {
        todo!()
    }

    async fn handle_set_language(&self, ctx: &AppContext, language: String) -> flare_core::error::Result<Response> {
        todo!()
    }

    async fn handle_close(&self, ctx: &AppContext) -> flare_core::error::Result<Response> {
        todo!()
    }
} 