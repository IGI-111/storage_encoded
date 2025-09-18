library;

use std::storage::storage_vec::*;
use std::string::*;
use std::codec::*;
use std::bytes_conversions::b256::*;
use std::bytes::*;
use std::storage::storage_api::*;
use std::storage::storable_slice::*;

// FIXME: ideally we'd want StorageEncoded<T> and to store the encoded type
// alongside the storage structure, but the compiler does not allow types with
// pointers anywhere in the storage right now, so to allow storing Vec and
// similar structures, we have to rely on the user decoding the right value and
// failing at runtime if not
pub struct StorageEncoded {}

impl StorageKey<StorageEncoded> {
    #[storage(read)]
    pub fn get<T>(self) -> T where
        T: AbiDecode
    {
        let bytes = read_slice(self.slot()).unwrap();
        let mut buf = BufferReader::from_parts(bytes.ptr(), bytes.len::<u8>());
        buf.decode()
    }

    #[storage(read, write)]
    pub fn set<T>(self, val: T) where
        T: AbiEncode
    {
        let buf = val.abi_encode(Buffer::new());
        let bytes = write_slice(self.slot(), buf.as_raw_slice());
    }
}

