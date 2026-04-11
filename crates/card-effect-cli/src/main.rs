use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};

use card_core::effect::text::card_effects_text;
use card_core::types::EffectKey;
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    let input = if args.len() >= 2 {
        // 从文件读取 JSON
        fs::read_to_string(&args[1])?
    } else {
        // 从标准输入读取 JSON
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        input
    };

    // 解析 JSON
    let effects: HashMap<EffectKey, Value> = serde_json::from_str(&input)?;

    // 转换为中文文字
    let texts = card_effects_text(&effects);

    // 输出结果，多个效果用换行分隔
    for text in texts {
        println!("{}", text);
    }

    Ok(())
}
