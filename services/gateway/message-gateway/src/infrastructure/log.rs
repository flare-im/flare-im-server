use flare_core::logs::{LogConfigBuilder, Logger};
use crate::infrastructure::config::get_config;

/// 初始化日志配置
pub fn init_log() -> anyhow::Result<()> {
    // 获取全局配置
    let config = get_config();
    let log_config = LogConfigBuilder::new().
        output_dir(config.log.output_dir.clone()).
        file_prefix(config.log.file_prefix.clone()).
        level(config.log.level.clone()).
        max_size(config.log.max_size).
        max_age(config.log.max_age).
        max_backups(config.log.max_backups).
        compress(config.log.compress).
        build();
    Logger::init(Some(log_config))?;
    Ok(())
}