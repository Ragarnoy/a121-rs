use crate::config::error::ConfigError;
use crate::sensor::error::SensorError;

#[derive(Debug)]
pub enum RadarError {
    SensorError(SensorError),
    ConfigError(ConfigError),
}
