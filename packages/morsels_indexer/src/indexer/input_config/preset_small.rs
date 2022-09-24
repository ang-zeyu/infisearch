use std::u32;

use super::{MorselsConfig, preset};

use serde_json::Value;

pub fn apply_config(config: &mut MorselsConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        100000000,
        true,
        u32::MAX,
        0,
        true,
        true,
        false
    );
}
