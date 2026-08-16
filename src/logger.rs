/*
 * Copyright (C) 2026 yuki
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::common;
use crate::fluent_args;
use crate::i18n::t_with_args;
use anyhow::{anyhow, Result};
use log::LevelFilter;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::{Encode, Write as EncodeWrite};
use log4rs::Handle;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

static LOG_HANDLE: OnceCell<Mutex<Handle>> = OnceCell::new();

fn parse_level(level_str: &str) -> LevelFilter {
    match level_str.to_uppercase().as_str() {
        "OFF" => LevelFilter::Off,
        "ERROR" => LevelFilter::Error,
        "WARN" => LevelFilter::Warn,
        "INFO" => LevelFilter::Info,
        "DEBUG" => LevelFilter::Debug,
        "TRACE" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}

#[derive(Debug)]
struct CompactEncoder;

impl Encode for CompactEncoder {
    fn encode(&self, w: &mut dyn EncodeWrite, record: &log::Record) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let h = (secs % 86400) / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;

        let level = match record.level() {
            log::Level::Error => 'E',
            log::Level::Warn => 'W',
            log::Level::Info => 'I',
            log::Level::Debug => 'D',
            log::Level::Trace => 'T',
        };

        let module = record
            .module_path()
            .and_then(|m| m.rsplit("::").next())
            .unwrap_or("?");

        Ok(writeln!(
            w,
            "{:02}:{:02}:{:02} {} {} {}",
            h,
            m,
            s,
            level,
            module,
            record.args()
        )?)
    }
}

fn build_config(level: LevelFilter) -> Result<Config> {
    let root = common::get_module_root();
    let log_path = root.join("logs/daemon.log");
    let archive_pattern = root.join("logs/daemon.{}.log");

    let roller = FixedWindowRoller::builder().build(archive_pattern.to_str().unwrap(), 3)?;
    let trigger = SizeTrigger::new(5 * 1024 * 1024); // 5MB
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let appender = RollingFileAppender::builder()
        .encoder(Box::new(CompactEncoder))
        .build(log_path, Box::new(policy))?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(appender)))
        .build(Root::builder().appender("logfile").build(level))?;

    Ok(config)
}

/// 初始化日志系统，启动时调用一次
pub fn init(level_str: &str) -> Result<()> {
    let level = parse_level(level_str);
    let config = build_config(level)?;
    let handle = log4rs::init_config(config)?;
    LOG_HANDLE
        .set(Mutex::new(handle))
        .map_err(|_| anyhow!("Logger already initialized"))?;
    Ok(())
}

/// 动态更新日志等级
pub fn update_level(level_str: &str) {
    let level = parse_level(level_str);
    if let Some(mutex) = LOG_HANDLE.get() {
        if let Ok(handle) = mutex.lock() {
            match build_config(level) {
                Ok(cfg) => {
                    handle.set_config(cfg);
                    log::debug!(
                        "{}",
                        t_with_args(
                            "log-level-updated",
                            &fluent_args!("level" => level.to_string())
                        )
                    );
                }
                Err(e) => eprintln!("Failed to rebuild logger config: {}", e),
            }
        }
    }
}
