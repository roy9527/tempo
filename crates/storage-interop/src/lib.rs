//! Storage interoperability primitives for Rust and Solidity contracts.

mod error;
mod layout;
mod packing;
mod slot;
mod storage;
mod types;
mod array;
mod bytes_like;
mod mapping;
mod vec;
mod runtime;

pub use error::{InteropError, Result};
pub use layout::{Handler, Layout, LayoutCtx, Packable, Storable, StorableType};
pub use packing::{
    FieldLocation, PackedSlot, calc_element_loc, calc_element_offset, calc_element_slot,
    calc_packed_slot_count, create_element_mask, extract_packed_value, insert_packed_value,
    zero_packed_value,
};
pub use slot::Slot;
pub use storage::{StorageKey, StorageOps};
pub use types::*;
pub use array::ArrayHandler;
pub use bytes_like::BytesLikeHandler;
pub use mapping::Mapping;
pub use vec::VecHandler;
pub use runtime::{PrecompileStorageProvider, RuntimeContext, RuntimeStorageOps, StorageMode};
#[cfg(feature = "revm")]
pub use runtime::RevmStorageProvider;
