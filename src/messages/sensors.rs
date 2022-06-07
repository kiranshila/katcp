//! Messages for dealing with sensor updates and configuration along with a [`Sensor`] type
//!
//! ## Example
//! This module is a little different from the others in that there is a core type, [`Sensor`] that is not
//! directly related to a message. The trick here is that sensor update messages ([`SensorValue`] and [`SensorStatus`]) have
//! values whose type depends on which sensor the value refers to. As Rust is statically typed, we can't determine how to deserialize
//! the `value` field without a priori knowing a mapping from sensor name to type. So, we leave those update messages unparsed, but implement
//! an update method for the [`Sensor`] type that can consume one of those update results.
//!
//! ```rust
//! use chrono::{TimeZone, Utc};
//! use katcp::messages::sensors::{Sensor, SensorUpdates, SensorValue, Status};
//!
//! // Make a new sensor that is Sensor<f32> (implied from the type of the value)
//! let mut pressure = Sensor::new("pump.pressure".to_owned(), Status::Unknown, Utc::now(), 0.0);
//! // Then, we get a new message that contains the sensor update (implied try_into from the if let)
//! let update = "#sensor-value 1427043968.954988 1 pump.pressure nominal 68.9"
//!     .try_into()
//!     .unwrap();
//! // The `update` variable here could contain multiple updates, but we'll just grab the on
//! if let SensorValue::Inform(SensorUpdates {
//!     timestamp,
//!     readings,
//! }) = update
//! {
//!     // This knows to serialize the reading into an f32 because of the type of the sensor
//!     pressure
//!         .update_from_reading(&timestamp, readings.first().unwrap())
//!         .unwrap();
//! }
//! // Sensor is now:
//! // Sensor {
//! //      name: "pump.pressure",
//! //      status: Nominal,
//! //      timestamp: 2015-03-22T17:06:08.954988002Z,
//! //      value: 68.9,
//! //  }
//! ```

use katcp_derive::{KatcpDiscrete, KatcpMessage};

use crate::{messages::common::from_argument_vec, prelude::*};

/// The core sensor type
///
/// The value of a sensor is generic to anything that impls [`crate::messages::common::KatcpArgument`]
#[derive(Debug, PartialEq, Eq)]
pub struct Sensor<T>
where
    T: KatcpArgument + Clone,
{
    name: String,
    status: Status,
    timestamp: KatcpTimestamp,
    value: T,
}

impl<T> Sensor<T>
where
    T: KatcpArgument<Err = KatcpError> + Clone,
{
    /// Constructor for a new sensor
    pub fn new(name: String, status: Status, timestamp: KatcpTimestamp, value: T) -> Self {
        Self {
            name,
            status,
            timestamp,
            value,
        }
    }

    /// Fetches the last value of the sensor
    pub fn value(&self) -> T {
        self.value.clone()
    }

    /// Fetches the last status of the sensor
    pub fn status(&self) -> Status {
        self.status
    }

    /// Fetches when the sensor was last updated
    pub fn last_updated(&self) -> KatcpTimestamp {
        self.timestamp
    }

    /// Update the sensor, requiring the updates status, timestamp, and value
    pub fn update(&mut self, status: &Status, timestamp: &KatcpTimestamp, value: &T) {
        self.status = *status;
        self.timestamp = *timestamp;
        self.value = value.clone();
    }

    pub fn update_from_reading(
        &mut self,
        timestamp: &KatcpTimestamp,
        reading: &SensorReading,
    ) -> Result<(), KatcpError> {
        if self.name != reading.name {
            return Err(KatcpError::Message(format!(
                "Tried to update sensor with name:{} with a reading of name:{}",
                self.name, reading.name
            )));
        }
        self.update(
            &reading.status,
            timestamp,
            &(T::from_argument(&reading.value)?),
        );
        Ok(())
    }
}

/// The katcp sensor statuses
#[derive(KatcpDiscrete, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status {
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

impl Status {
    /// Returns if a given [`Status`] is valid according to the spec
    pub fn is_valid(self) -> bool {
        matches!(self, Self::Nominal | Self::Warn | Self::Error)
    }
}

#[derive(Debug, PartialEq)]
/// The data of a [`SensorList`] inform message.
/// You would use this information to design a [`Sensor`] type
pub struct SensorListInform {
    /// is the name of the sensor in dotted notation. This notation allows a virtual hierarchy of sensors to
    /// be represented; e.g. a name might be rfe0.temperature.
    pub name: String,
    /// is a human-readable description of the information provided by the sensor.
    pub description: String,
    /// is a human-readable string containing a short form of the units for the sensor value. May be blank
    /// if there are no suitable units. Examples: "kg", "packet count", "m/s". Should be suitable for display
    /// next to the value in a user interface
    pub units: String,
    /// The params themselves. The meaning of the params depend on the types
    ///
    /// # Notes
    /// Note that the specifying the optional error and warning ranges for integer or float sensors does
    /// not relieve the device from setting the correct status on sensors itself; it is only meant to provide
    /// extra information to users of a device. The device exposing the sensor must ensure that the way it
    /// reports sensor status is consistent with the ranges reported by the [`SensorList`] inform. If it is not
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
    pub params: ArgumentVec,
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
/// The messages to query the available sensors
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
    /// seconds. The period will be specified using seconds as an f32.
    /// May be implementedmented for sensors of any type.
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

impl ToKatcpArguments for SamplingStrategy {
    fn to_arguments(&self) -> Vec<String> {
        match self {
            SamplingStrategy::Auto => vec!["auto".to_owned()],
            SamplingStrategy::None => vec!["none".to_owned()],
            SamplingStrategy::Period { period } => vec!["period".to_owned(), period.to_argument()],
            SamplingStrategy::Event => vec!["event".to_owned()],
            SamplingStrategy::Differential { difference } => {
                vec!["differential".to_owned(), difference.to_argument()]
            }
            SamplingStrategy::EventRate {
                shortest_period,
                longest_period,
            } => vec![
                "event-rate".to_owned(),
                shortest_period.to_argument(),
                longest_period.to_argument(),
            ],
            SamplingStrategy::DifferentialRate {
                difference,
                shortest_period,
                longest_period,
            } => vec![
                "differential-rate".to_owned(),
                difference.to_argument(),
                shortest_period.to_argument(),
                longest_period.to_argument(),
            ],
        }
    }
}

impl FromKatcpArguments for SamplingStrategy {
    type Err = KatcpError;
    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, KatcpError> {
        let strat = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        Ok(match strat.as_str() {
            "auto" => SamplingStrategy::Auto,
            "none" => SamplingStrategy::None,
            "period" => SamplingStrategy::Period {
                period: f32::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
            },
            "event" => SamplingStrategy::Event,
            "differential" => SamplingStrategy::Differential {
                difference: f32::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
            },
            "event-rate" => SamplingStrategy::EventRate {
                shortest_period: f32::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
                longest_period: f32::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
            },
            "differential-rate" => SamplingStrategy::DifferentialRate {
                difference: f32::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?,
                shortest_period: f32::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
                longest_period: f32::from_argument(
                    strings.next().ok_or(KatcpError::MissingArgument)?,
                )?,
            },
            _ => return Err(KatcpError::BadArgument),
        })
    }
}

#[derive(Debug, PartialEq)]
/// The type representing a sensor sampling request
pub struct SamplingRequest {
    /// is the name of a single sensor. For bulk setting a comma-separated list of many sensor names can be used if the server supports the `B` flag
    pub names: String,
    /// pecifies a sampling strategy and is one of the strategies described in [`SamplingStrategy`]
    /// If no strategy is
    /// specified, the current strategy and parameters are left unchanged and just reported in the reply. This
    /// querying of a strategy is only applicable when specifying a single sensor name, not a list of names.
    pub strategy: Option<SamplingStrategy>,
}

impl ToKatcpArguments for SamplingRequest {
    fn to_arguments(&self) -> Vec<String> {
        let mut prelude = vec![self.names.to_argument()];
        if let Some(strat) = &self.strategy {
            prelude.append(&mut strat.to_arguments());
            prelude
        } else {
            prelude
        }
    }
}

impl FromKatcpArguments for SamplingRequest {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let names = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        // If the next string is empty, we don't care, but a BadArgument is a real error we want to send up
        match SamplingStrategy::from_arguments(strings) {
            Ok(strategy) => Ok(Self {
                names,
                strategy: Some(strategy),
            }),
            Err(e) => match e {
                e @ KatcpError::BadArgument => Err(e),
                _ => Ok(Self {
                    names,
                    strategy: None,
                }),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
/// The Reply type for [`SensorSampling`]
pub struct SamplingReply {
    pub names: String,
    pub strategy: SamplingStrategy,
}

impl ToKatcpArguments for SamplingReply {
    fn to_arguments(&self) -> Vec<String> {
        let mut prelude = vec![self.names.to_argument()];
        prelude.append(&mut self.strategy.to_arguments());
        prelude
    }
}
impl FromKatcpArguments for SamplingReply {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let names = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let strategy = SamplingStrategy::from_arguments(strings)?;
        Ok(Self { names, strategy })
    }
}

#[derive(KatcpMessage, Debug, PartialEq)]
/// The messages that control how sensors are sampled
pub enum SensorSampling {
    Request(SamplingRequest),
    Reply(SamplingReply),
}

#[derive(Debug, PartialEq, Eq)]
/// A complete sensor reading, returned by [`SensorValue`] and [`SensorStatus`]
pub struct SensorReading {
    pub name: String,
    pub status: Status,
    /// A bare sensor reading will be kept as a string as its type
    /// is dependent on the value of `name`
    pub value: String,
}

impl FromKatcpArguments for SensorReading {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let name = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let status = Status::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let value = String::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        Ok(Self {
            name,
            status,
            value,
        })
    }
}

impl ToKatcpArguments for SensorReading {
    fn to_arguments(&self) -> Vec<String> {
        vec![
            self.name.to_argument(),
            self.status.to_argument(),
            self.value.to_argument(),
        ]
    }
}

#[derive(Debug, PartialEq, Eq)]
/// A timestamped collection of [`SensorReading`]s
pub struct SensorUpdates {
    pub timestamp: KatcpTimestamp,
    pub readings: Vec<SensorReading>,
}

impl FromKatcpArguments for SensorUpdates {
    type Err = KatcpError;

    fn from_arguments(strings: &mut impl Iterator<Item = String>) -> Result<Self, Self::Err> {
        let timestamp =
            KatcpTimestamp::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let num_sensors = u32::from_argument(strings.next().ok_or(KatcpError::MissingArgument)?)?;
        let mut readings = vec![];
        for _ in 1..=num_sensors {
            readings.push(SensorReading::from_arguments(strings)?);
        }
        Ok(Self {
            timestamp,
            readings,
        })
    }
}

impl ToKatcpArguments for SensorUpdates {
    fn to_arguments(&self) -> Vec<String> {
        let mut prelude = vec![
            self.timestamp.to_argument(),
            (self.readings.len() as u32).to_argument(),
        ];
        prelude.append(
            &mut self
                .readings
                .iter()
                .flat_map(|r| r.to_arguments())
                .collect(),
        );
        prelude
    }
}

#[derive(KatcpMessage, Debug, PartialEq, Eq)]
/// The messages involving directly querying a sensor's value
pub enum SensorValue {
    /// Before sending a reply, the sensor-value request will send a number of sensor-value inform messages. If
    /// no name parameter is sent the sensor-value request will return a sensor value for each sensor available on
    /// the device using a set of sensor-value inform messages. If a name parameter is specified, only an inform
    /// message for that sensor will be sent. On success the first reply parameter after the status code will contain
    /// the number of inform messages generated by this request. If the name parameter does not correspond to
    /// a sensor on the device, a fail reply should be sent.
    Request {
        name: Option<String>,
    },
    Reply(IntReply),
    /// The sensor-value inform message has the same structure as the asynchronous [`sensor-status`] inform except
    /// for the message name. The message name is used to determine whether the sensor value is being reported
    /// in response to a sensor-value request or as a result of sensor sampling.
    Inform(SensorUpdates),
}

#[derive(KatcpMessage, Debug, PartialEq, Eq)]
/// The async sensor status update message
pub enum SensorStatus {
    /// A sensor-status inform should be sent whenever the sensor sampling set up by the client dictates. The
    /// sensor-status inform message has the same structure as the [`SensorValue`] inform except for the message
    /// name. The message name is used to determine whether the sensor value is being reported in response to
    /// a sensor-value request or as a result of sensor sampling
    Inform(SensorUpdates),
}

#[cfg(test)]
mod sensor_tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::messages::common::roundtrip_test;

    #[test]
    fn test_sensor() {
        let mut pump_pressure = Sensor::new(
            "pump.pressure".to_owned(),
            Status::Nominal,
            Utc::now(),
            3.15,
        );
        assert_eq!(pump_pressure.value(), 3.15);
        pump_pressure.update(&Status::Warn, &Utc::now(), &90000.0);
        assert_eq!(pump_pressure.status(), Status::Warn);
        assert_eq!(pump_pressure.value(), 90000.0);
        assert!(pump_pressure.status().is_valid());
    }

    #[test]
    fn status_validity() {
        assert!(!Status::Unknown.is_valid());
        assert!(Status::Nominal.is_valid());
        assert!(Status::Warn.is_valid());
        assert!(Status::Error.is_valid());
        assert!(!Status::Failure.is_valid());
        assert!(!Status::Unreachable.is_valid());
        assert!(!Status::Inactive.is_valid());
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

    #[test]
    fn test_sensor_sampling() {
        roundtrip_test(SensorSampling::Request(SamplingRequest {
            names: "wind-speed".to_owned(),
            strategy: Some(SamplingStrategy::Auto),
        }));
        roundtrip_test(SensorSampling::Request(SamplingRequest {
            names: "wind-speed".to_owned(),
            strategy: Some(SamplingStrategy::None),
        }));
        roundtrip_test(SensorSampling::Request(SamplingRequest {
            names: "wind-speed".to_owned(),
            strategy: None,
        }));
        roundtrip_test(SensorSampling::Request(SamplingRequest {
            names: "wind-speed".to_owned(),
            strategy: Some(SamplingStrategy::Period { period: 1.0 }),
        }));
        roundtrip_test(SensorSampling::Request(SamplingRequest {
            names: "wind-speed".to_owned(),
            strategy: Some(SamplingStrategy::DifferentialRate {
                difference: 10.5,
                shortest_period: 3.1,
                longest_period: 15.0,
            }),
        }));
        roundtrip_test(SensorSampling::Reply(SamplingReply {
            names: "wind-speed".to_owned(),
            strategy: SamplingStrategy::EventRate {
                shortest_period: 3.15,
                longest_period: 2.71,
            },
        }));
        roundtrip_test(SensorSampling::Reply(SamplingReply {
            names: "wind-speed".to_owned(),
            strategy: SamplingStrategy::Differential { difference: 420.69 },
        }));
    }

    #[test]
    fn test_sensor_value() {
        roundtrip_test(SensorValue::Request { name: None });
        roundtrip_test(SensorValue::Request {
            name: Some("antennas.1.pitch".to_owned()),
        });
        roundtrip_test(SensorValue::Reply(IntReply::Ok { num: 10 }));
        roundtrip_test(SensorValue::Reply(IntReply::Error {
            ret_code: RetCode::Invalid,
            message: "Uh oh".to_owned(),
        }));
        roundtrip_test(SensorValue::Inform(SensorUpdates {
            timestamp: Utc.timestamp(1654553033, 0),
            readings: vec![
                SensorReading {
                    name: "big-fat-motor.current".to_owned(),
                    status: Status::Nominal,
                    value: "0.813".to_owned(),
                },
                SensorReading {
                    name: "big-fat-motor.voltage".to_owned(),
                    status: Status::Nominal,
                    value: "24.1".to_owned(),
                },
            ],
        }));
    }

    #[test]
    fn test_updating_sensor_values() {
        let mut pump_pressure = Sensor::new(
            "pump.pressure".to_owned(),
            Status::Nominal,
            Utc::now(),
            3.15,
        );
        let new_time = Utc::now();
        let incoming_message: SensorValue = format!(
            "#sensor-value {} 1 pump.pressure warn 8.73",
            new_time.to_argument()
        )
        .as_str()
        .try_into()
        .unwrap();
        if let SensorValue::Inform(SensorUpdates {
            timestamp,
            readings,
        }) = incoming_message
        {
            // This knows to serialize the reading into an f32 because of the type of the sensor
            pump_pressure
                .update_from_reading(&timestamp, readings.first().unwrap())
                .unwrap();
            assert_eq!(pump_pressure.value(), 8.73);
            assert_eq!(pump_pressure.status(), Status::Warn);
        } else {
            panic!()
        }
    }

    #[test]
    fn test_sensor_status() {
        roundtrip_test(SensorStatus::Inform(SensorUpdates {
            timestamp: Utc.timestamp(1654553033, 0),
            readings: vec![
                SensorReading {
                    name: "big-fat-motor.current".to_owned(),
                    status: Status::Nominal,
                    value: "0.813".to_owned(),
                },
                SensorReading {
                    name: "big-fat-motor.voltage".to_owned(),
                    status: Status::Nominal,
                    value: "24.1".to_owned(),
                },
            ],
        }));
    }
}
