use super::{InfiConfig, preset};

use serde_json::Value;

pub fn apply_config(config: &mut InfiConfig, json_config: &Value) {
    preset::apply_preset_override(
        config,
        json_config,
        2,
        false,
        4096000,
        0
    );
}
