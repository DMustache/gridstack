use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::services::rooms::entities::{
    RoomVersionRules, RoomVersionRulesContract,
    versions::{self, v1::RoomVersionRulesV1},
};

pub mod v1;
pub mod v10;
pub mod v11;
pub mod v12;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;
pub mod v7;
pub mod v8;
pub mod v9;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    EnumString,
    Display,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
)]
pub enum RoomVersion {
    #[default]
    #[serde(rename = "1")]
    #[strum(serialize = "1")]
    V1 = 1,
    #[serde(rename = "2")]
    #[strum(serialize = "2")]
    V2 = 2,
    #[serde(rename = "3")]
    #[strum(serialize = "3")]
    V3 = 3,
    #[serde(rename = "4")]
    #[strum(serialize = "4")]
    V4 = 4,
    #[serde(rename = "5")]
    #[strum(serialize = "5")]
    V5 = 5,
    #[serde(rename = "6")]
    #[strum(serialize = "6")]
    V6 = 6,
    #[serde(rename = "7")]
    #[strum(serialize = "7")]
    V7 = 7,
    #[serde(rename = "8")]
    #[strum(serialize = "8")]
    V8 = 8,
    #[serde(rename = "9")]
    #[strum(serialize = "9")]
    V9 = 9,
    #[serde(rename = "10")]
    #[strum(serialize = "10")]
    V10 = 10,
    #[serde(rename = "11")]
    #[strum(serialize = "11")]
    V11 = 11,
    #[serde(rename = "12")]
    #[strum(serialize = "12")]
    V12 = 12,
}

impl RoomVersion {
    pub const fn as_number(self) -> u8 {
        self as u8
    }

    pub fn requires_creator_field(self) -> bool {
        self <= Self::V11
    }

    pub fn supports_additional_creators(self) -> bool {
        self >= Self::V12
    }

    pub fn rules(self) -> RoomVersionRules {
        match self {
            Self::V1 => RoomVersionRulesV1::rules(),
            Self::V2 => versions::v2::rules(),
            Self::V3 => versions::v3::rules(),
            Self::V4 => versions::v4::rules(),
            Self::V5 => versions::v5::rules(),
            Self::V6 => versions::v6::rules(),
            Self::V7 => versions::v7::rules(),
            Self::V8 => versions::v8::rules(),
            Self::V9 => versions::v9::rules(),
            Self::V10 => versions::v10::rules(),
            Self::V11 => versions::v11::rules(),
            Self::V12 => versions::v12::rules(),
        }
    }
}
