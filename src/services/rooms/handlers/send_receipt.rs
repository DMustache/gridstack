use serde::Serialize;

use crate::services::rooms::{
    entities::{RoomReceiptType, SendReceiptCommand},
    errors::RoomsApplicationError,
};

pub use crate::services::rooms::entities::SendReceiptRequestDto as SendReceiptInfo;

#[derive(Clone, Debug, Default, Serialize)]
pub struct SendReceiptView {}

impl TryFrom<(&str, String, SendReceiptInfo)> for SendReceiptCommand {
    type Error = RoomsApplicationError;

    fn try_from(value: (&str, String, SendReceiptInfo)) -> Result<Self, Self::Error> {
        let (receipt_type, event_id, request) = value;
        let receipt_type =
            RoomReceiptType::parse(receipt_type).ok_or(RoomsApplicationError::InvalidParameter)?;

        Ok(Self {
            receipt_type,
            event_id,
            thread_id: request.thread_id,
        })
    }
}
