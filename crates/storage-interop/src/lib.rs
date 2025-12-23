//! Storage interoperability primitives for Rust and Solidity contracts.

mod error;
mod layout;
mod packing;
mod storage;
mod types;

pub use error::{InteropError, Result};
pub use layout::{Handler, Layout, LayoutCtx, Packable, Storable, StorableType};
pub use packing::{
    FieldLocation, PackedSlot, calc_element_loc, calc_element_offset, calc_element_slot,
    calc_packed_slot_count, create_element_mask, extract_packed_value, insert_packed_value,
    zero_packed_value,
};
pub use storage::{Slot, StorageKey, StorageOps};
pub use types::*;
