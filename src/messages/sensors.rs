use katcp_derive::{KatcpDiscrete, KatcpMessage};

use super::{common::from_argument_vec, core::IntReply};
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
/// The data of a [`SensorList`] inform message
pub struct SensorListInform {
    /// is the name of the sensor in dotted notation. This notation allows a virtual hierarchy of sensors to
    /// be represented; e.g. a name might be rfe0.temperature.
    name: String,
    /// is a human-readable description of the information provided by the sensor.
    description: String,
    /// is a human-readable string containing a short form of the units for the sensor value. May be blank
    /// if there are no suitable units. Examples: "kg", "packet count", "m/s". Should be suitable for display
    /// next to the value in a user interface
    units: String,
    /// The params themselves. The meaning of the params depend on the types
    ///
    /// # Notes
    /// Note that the specifying the optional error and warning ranges for integer or float sensors does
    /// not relieve the device from setting the correct status on sensors itself; it is only meant to provide
    /// extra information to users of a device. The device exposing the sensor must ensure that the way it
    /// reports sensor status is consistent with the ranges reported by the #sensor-list inform. If it is not
    /// possible to do so, the ranges should be omitted.
    ///
    /// Any sensor value (assuming the sensor status is not unknown, failure, unreachable or inactive) x :
    /// nominal-min ≤ x ≤ nominal-max should be accompanied by a nominal sensor state. If only
    /// nominal-min and nominal-max are specified, Values outside this range may be accompanied
    /// by warning or error states. If warn-min and warn-max are also specified, values of x such that
    /// warn-min ≤ x < nominal-min or nominal-max < x ≤ warn-max should be accompanied by a
    /// warning status, while values outside these ranges should be be accompanied by an error status.
    ///
    /// # Type Information
    /// ## Integer
    /// `[nominal-min nominal-max [warn-min warn-max]]`
    ///
    /// ## Float
    /// `[nominal-min nominal-max [warn-min warn-max]]`
    ///
    /// ## Discrete
    /// list of available options
    ///
    /// ## Boolean, Timestamp, Address, String
    /// No additional parameters
    params: ArgumentVec,
}

impl ToKatcpArguments for SensorListInform {
    fn to_arguments(&self) -> Vec<String> {
        let mut prelude = vec![
            self.name.to_argument(),
            self.description.to_argument(),
            self.units.to_argument(),
        ];
        prelude.push(self.params.to_string());
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
        let params = from_argument_vec(&ty, strings)?;
        Ok(Self {
            name,
            description,
            units,
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

#[derive(Debug, PartialEq)]
/// The sampling strategy (and associated params) for [`SensorSampling`]
pub enum SamplingStrategy {
    /// Report the sensor value when convenient for
    /// the device. This should never be equivalent to
    /// the none strategy
    Auto,
    /// Do not report the sensor value.
    None,
    /// Report the value approximately every period
    /// seconds. The period will be specified using
    /// the timestamp data format. May be
    /// implementedmented for sensors of any type.
    Period { period: f32 },
    /// Report the value whenever it changes. May
    /// be implemented for sensors of any type. For
    /// float sensors the device will have to determine
    /// how much of a shift constitutes a real
    /// change.
    Event,
    /// Report the value when it changes by more than
    /// difference from the last reported value. May
    /// only be implemented for float and integer
    /// sensors. The difference is formatted as a
    /// float for float sensors and an integer for
    /// integer sensors.
    Differential { difference: f32 },
    /// Report the value whenever it changes or if
    /// more than longest-period seconds have
    /// passed since the last reported update. However,
    /// do not report the value until at
    /// least shortest-period seconds have passed
    /// since the last reported update. The behaviour
    /// if shortest-period is greater than
    /// longest-period is undefined.
    EventRate {
        shortest_period: f32,
        longest_period: f32,
    },
    /// Report the value whenever it changes by
    /// more than difference from the last reported
    /// value or if more than longest-period seconds
    /// have passed since the last reported update.
    /// However, do not report the value until at
    /// least shortest-period seconds have passed
    /// since the last reported update. The behaviour
    /// if shortest-period is greater than longest-period
    /// is undefined. May only be implemented for float
    /// and integer sensors. The difference is formatted
    /// as a float for float sensors and an integer for integer sensors.
    DifferentialRate {
        difference: f32,
        shortest_period: f32,
        longest_period: f32,
    },
}

impl ToKatcpArgument for SamplingStrategy {
    fn to_argument(&self) -> String {
        match self {
            SamplingStrategy::Auto => "auto",
            SamplingStrategy::None => "none",
            SamplingStrategy::Period { .. } => "period",
            SamplingStrategy::Event => "event",
            SamplingStrategy::Differential { .. } => "differential",
            SamplingStrategy::EventRate { .. } => "event-rate",
            SamplingStrategy::DifferentialRate { .. } => "differential-rate",
        }
        .to_owned()
    }
}

#[derive(Debug, PartialEq)]
/// The type representing a sensor sampling request
pub struct SamplingRequest {
    names: String,
    strategy: Option<SamplingStrategy>,
}

// FIXME
impl ToKatcpArguments for SamplingRequest {
    fn to_arguments(&self) -> Vec<String> {
        let mut prelude = vec![self.names.to_argument()];
        if let Some(strat) = &self.strategy {
            prelude.push(strat.to_argument());
            let mut extra = match strat {
                SamplingStrategy::Auto => vec![],
                SamplingStrategy::None => vec![],
                SamplingStrategy::Period { period } => vec![period.to_argument()],
                SamplingStrategy::Event => vec![],
                SamplingStrategy::Differential { difference } => vec![difference.to_argument()],
                SamplingStrategy::EventRate {
                    shortest_period,
                    longest_period,
                } => vec![shortest_period.to_argument(), longest_period.to_argument()],
                SamplingStrategy::DifferentialRate {
                    difference,
                    shortest_period,
                    longest_period,
                } => vec![
                    difference.to_argument(),
                    shortest_period.to_argument(),
                    longest_period.to_argument(),
                ],
            };
            prelude.append(&mut extra);
            prelude
        } else {
            prelude
        }
    }
}

// FIXME
impl FromKatcpArguments for SamplingRequest {
    type Err = KatcpError;

    fn from_arguments(_strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        todo!()
    }
}

#[derive(KatcpMessage, Debug, PartialEq)]
pub enum SensorSampling {
    Request(SamplingRequest),
}

#[cfg(test)]
mod sensor_tests {
    use super::*;
    use crate::messages::common::roundtrip_test;

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

    #[test]
    fn test_sensor_list() {
        roundtrip_test(SensorList::Request { name: None });
        roundtrip_test(SensorList::Request {
            name: Some("rfe0.temperature".to_owned()),
        });
        roundtrip_test(SensorList::Reply(IntReply::Ok { num: 420 }));
        roundtrip_test(SensorList::Inform(SensorListInform {
            name: "rfe0.temperature".to_owned(),
            description: "The temperature of rfe0".to_owned(),
            units: "Kelvin".to_owned(),
            params: ArgumentVec::Float(vec![123.234, 0.2, 12., -122e05]),
        }));
    }
}
