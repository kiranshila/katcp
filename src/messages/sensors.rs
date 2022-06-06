use chrono::{DateTime, Utc};
use katcp_derive::{KatcpDiscrete, KatcpMessage};

use super::core::IntReply;
use crate::prelude::*;

/// The valid katcp "sensor" statuses
#[derive(KatcpDiscrete, Debug, PartialEq, Eq)]
pub enum SensorStatus {
    /// The sensor is in the process of being initialized and no value has yet been
    /// seen. Sensors should not remain in this state indefinitely.
    Unknown,
    /// The sensor reading is within the expected range of nominal operating values.
    Nominal,
    /// The sensor reading is outside the nominal operating range.
    Warn,
    /// The sensor reading indicates a critical condition for the device.
    Error,
    /// Taking a sensor reading failed and seems unlikely to succeed in future
    /// without maintenance.
    Failure,
    /// The sensor could not be reached. This should only be used by a server that
    /// is proxying the sensor for another KATCP device. A sensor that is read by
    /// the server from a source other than another KATCP device should not be set
    /// to this status.
    Unreachable,
    /// The sensor is inactive; while the sensor does not provide a valid value, this
    /// status does not represent a failure condition. It could indicate that optional
    /// sensing hardware is not connected; in multi-mode devices it may indicate
    /// that a particular sensor is not applicable to the current mode of operation.
    Inactive,
}

impl SensorStatus {
    /// Returns if a given `SensorStatus` is valid
    pub fn is_valid(self) -> bool {
        matches!(
            self,
            SensorStatus::Nominal | SensorStatus::Warn | SensorStatus::Error
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum SensorListParams {
    Integer(Vec<i32>),
    Float(Vec<f32>),
    Boolean(Vec<bool>),
    Timestamp(Vec<DateTime<Utc>>),
    String(Vec<String>),
    Discrete(Vec<String>),
}

impl ToKatcpArguments for SensorListParams {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            SensorListParams::Integer(v) => v.iter().map(|e| e.to_argument()).collect(),
            SensorListParams::Float(v) => v.iter().map(|e| e.to_argument()).collect(),
            SensorListParams::Boolean(v) => v.iter().map(|e| e.to_argument()).collect(),
            SensorListParams::Timestamp(v) => v.iter().map(|e| e.to_argument()).collect(),
            SensorListParams::String(v) => v.iter().map(|e| e.to_argument()).collect(),
            SensorListParams::Discrete(v) => v.iter().map(|e| e.to_argument()).collect(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SensorListInform {
    name: String,
    description: String,
    units: String,
    ty: ArgumentType,
    params: SensorListParams,
}

impl ToKatcpArguments for SensorListInform {
    fn to_arguments(&self) -> Vec<String> {
        let mut prelude = vec![
            self.name.to_argument(),
            self.description.to_argument(),
            self.units.to_argument(),
            self.ty.to_argument(),
        ];
        // Why oh why does append not return the result
        prelude.append(&mut self.params.to_arguments());
        prelude
    }
}

impl FromKatcpArguments for SensorListInform {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let name = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let description =
            String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let units = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let ty = ArgumentType::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let params = match ty {
            ArgumentType::Boolean => SensorListParams::Boolean(
                strings
                    .map(bool::from_argument)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ArgumentType::Integer => SensorListParams::Integer(
                strings
                    .map(i32::from_argument)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ArgumentType::Float => SensorListParams::Float(
                strings
                    .map(f32::from_argument)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ArgumentType::Timestamp => SensorListParams::Timestamp(
                strings
                    .map(DateTime::<Utc>::from_argument)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ArgumentType::Discrete => SensorListParams::Discrete(
                strings
                    .map(String::from_argument)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            ArgumentType::Address => todo!(),
            ArgumentType::String => SensorListParams::String(
                strings
                    .map(String::from_argument)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        };
        Ok(Self {
            name,
            description,
            units,
            ty,
            params,
        })
    }
}

// Sensor Sampling
#[derive(KatcpMessage, Debug, PartialEq)]
pub enum SensorList {
    /// Before sending a reply, the sensor-list request will send a number of sensor-list inform messages. If no
    /// name parameter is sent the sensor-list request will return a sensor-list inform message for each sensor
    /// available on the device. If a name parameter is specified, only an inform message for that sensor will
    /// be sent. On success the first reply parameter after the status code will contain the number of inform
    /// messages generated by this request. If the name parameter does not correspond to a sensor on the device,
    /// a fail reply should be sent.
    Request {
        name: Option<String>,
    },
    Inform(SensorListInform),
    Reply(IntReply),
}

#[cfg(test)]
mod sensor_tests {
    use super::*;

    #[test]
    fn status_validity() {
        assert!(!SensorStatus::Unknown.is_valid());
        assert!(SensorStatus::Nominal.is_valid());
        assert!(SensorStatus::Warn.is_valid());
        assert!(SensorStatus::Error.is_valid());
        assert!(!SensorStatus::Failure.is_valid());
        assert!(!SensorStatus::Unreachable.is_valid());
        assert!(!SensorStatus::Inactive.is_valid());
    }
}
