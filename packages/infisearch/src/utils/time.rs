use std::time::Instant;

use log::info;

pub fn print_time_elapsed(instant: &Option<Instant>, extra_message: &str) {
    if let Some(instant) = instant {
        let elapsed = instant.elapsed().as_secs_f64();
        info!("({}) {} mins {} seconds elapsed.", extra_message, (elapsed as u32) / 60, elapsed % 60.0);
    }
}
