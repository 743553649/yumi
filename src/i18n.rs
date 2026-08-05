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

use fluent::bundle::FluentBundle;
use fluent::{FluentResource, FluentArgs};
use intl_memoizer::concurrent::IntlLangMemoizer;
use once_cell::sync::Lazy;
use std::fs;
use std::sync::RwLock;
use crate::common;

// 全局静态变量，存储当前的语言包
static BUNDLE: Lazy<RwLock<FluentBundle<FluentResource, IntlLangMemoizer>>> = Lazy::new(|| {
    let bundle = FluentBundle::new_concurrent(vec!["zh".parse().unwrap()]);
    RwLock::new(bundle)
});

/// 核心函数：加载指定语言的文件
fn load_bundle(lang: &str) -> Result<FluentBundle<FluentResource, IntlLangMemoizer>, anyhow::Error> {
    // 1. 获取模块根目录
    let root = common::get_module_root();
    
    // 2. 拼接完整路径
    let ftl_path = root.join(format!("config/i18n/{}.ftl", lang));

    log::info!("[i18n] Attempting to load language '{}' from: {:?}", lang, ftl_path);

    // 3. 读取文件内容
    let ftl_string = fs::read_to_string(&ftl_path)
        .map_err(|e| anyhow::anyhow!("Failed to read FTL file {:?}: {}", ftl_path, e))?;

    // 4. 解析资源
    let resource = FluentResource::try_new(ftl_string)
        .map_err(|e| anyhow::anyhow!("Failed to parse FTL resource {:?}: {:?}", ftl_path, e))?;

    let lang_id = lang.parse().unwrap_or_else(|_| "zh".parse().unwrap());
    let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);

    bundle.add_resource(resource)
        .map_err(|e| anyhow::anyhow!("Failed to add FTL resource: {:?}", e))?;

    Ok(bundle)
}

/// 对外接口：切换语言
pub fn load_language(lang: &str) {
    log::info!("[i18n] Request to switch language to: '{}'", lang);
    
    match load_bundle(lang) {
        Ok(new_bundle) => {
            let mut bundle_lock = BUNDLE.write().unwrap_or_else(|e| e.into_inner());
            *bundle_lock = new_bundle;
            log::info!("[i18n] Successfully loaded and switched to language: {}", lang);
        }
        Err(e) => {
            // 如果加载失败，不会覆盖旧的语言包
            log::error!("[i18n] Failed to load language '{}': {}. Keeping previous language.", lang, e);
        }
    }
}

fn format_missing_key(key: &str) -> String {
    log::debug!("[i18n] Missing translation key: '{}'", key);
    key.split(|c| c == '-' || c == '_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 获取翻译文本
pub fn t(key: &str) -> String {
    let bundle = BUNDLE.read().unwrap_or_else(|e| e.into_inner());
    let msg = match bundle.get_message(key) {
        Some(msg) => msg,
        None => return format_missing_key(key),
    };
    let pattern = match msg.value() {
        Some(pattern) => pattern,
        None => return format_missing_key(key),
    };

    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, None, &mut errors);
    
    if errors.is_empty() {
        value.to_string()
    } else {
        log::debug!("[i18n] Failed to format message for key '{}': {:?}", key, errors);
        format_missing_key(key)
    }
}

/// 获取带参数的翻译文本
pub fn t_with_args(key: &str, args: &FluentArgs) -> String {
    let bundle = BUNDLE.read().unwrap_or_else(|e| e.into_inner());
    let msg = match bundle.get_message(key) {
        Some(msg) => msg,
        None => return format_missing_key(key), 
    };
    let pattern = match msg.value() {
        Some(pattern) => pattern,
        None => return format_missing_key(key), 
    };

    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, Some(args), &mut errors);
    
    if errors.is_empty() {
        value.to_string()
    } else {
        log::debug!("[i18n] Failed to format message for key '{}': {:?}", key, errors);
        format_missing_key(key)
    }
}

#[macro_export] 
macro_rules! fluent_args {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut args = fluent::FluentArgs::new();
        $(
            args.set($key, fluent::FluentValue::from($value));
        )*
        args
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_missing_key() {
        assert_eq!(format_missing_key("touch-boost-channel-disconnected"), "Touch Boost Channel Disconnected");
        assert_eq!(format_missing_key("scheduler_module_started"), "Scheduler Module Started");
        assert_eq!(format_missing_key("unknown_key"), "Unknown Key");
    }
}