use anyhow::Result;
use common::config::{Config, Environment};
use once_cell::sync::OnceCell;
use std::path::PathBuf;

static CONFIG: OnceCell<Config> = OnceCell::new();

/// 初始化全局配置
pub fn init_config(env: Environment) -> Result<()> {
    let config = Config::from_env_file::<PathBuf>(env)?;
    CONFIG.set(config).map_err(|_| anyhow::anyhow!("Config already initialized"))?;
    Ok(())
}

/// 获取全局配置
pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}