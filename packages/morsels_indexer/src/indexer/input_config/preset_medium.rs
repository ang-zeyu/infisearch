use super::{MorselsConfig, preset};

use serde_json::Value;

pub fn apply_config(config: &mut MorselsConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        2,
        false,
        4096000,
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
        100000000,
        true,
        4096000,
        0,
        false,
        false,
        true
    );
}
