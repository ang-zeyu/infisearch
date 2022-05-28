use super::{MorselsConfig, preset};

use serde_json::Value;

pub fn apply_config(config: &mut MorselsConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        1,
        false,
        100000,
        1048576,
        true,
        false
    );
}

pub fn apply_source_file_config(config: &mut MorselsConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        u32::MAX,
        true,
        100000,
        1048576,
        false,
        false
    );
}
