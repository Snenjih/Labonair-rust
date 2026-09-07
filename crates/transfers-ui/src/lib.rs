//! GPUI presentation for the transfer capability.
//!
//! The view owns no transport or transfer lifecycle logic. It renders
//! snapshots from [`labonair_transfers::TransferRegistry`] and sends user
//! actions through the injected [`labonair_transfers::TransferService`].

mod status_item;
mod view;

pub use status_item::{status_item_registration, TransfersStatusItem};
pub use view::{TransferUiEvent, TransfersView};
