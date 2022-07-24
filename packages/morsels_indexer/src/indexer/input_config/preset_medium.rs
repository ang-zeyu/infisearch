use super::{MorselsConfig, preset};

use serde_json::Value;

pub fn apply_config(config: &mut MorselsConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        3,
        false,
        2097151,
        0,
        false,
        true,
        true
    );
}

pub fn apply_source_file_config(config: &mut MorselsConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        u32::MAX,
        true,
        u32::MAX,
        0,
        false,
        false,
        true
    );
}
