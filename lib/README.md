# StorageEncoded

A Sway library that enables direct storage access for encoded data in Fuel smart contracts, allowing off-chain reads without executing getter functions or simulating transactions.

## Overview

`StorageEncoded` provides a storage pattern for Sway smart contracts that allows direct off-chain reading of contract state. By storing ABI-encoded data in a predictable format, you can retrieve complex types directly from contract storage using the companion Rust library—no transaction simulation or FuelVM execution required.

## Key Benefits

- **Direct Storage Access**: Read contract state off-chain without executing getter functions
- **Performance**: Much faster than simulating getter functions for off-chain reads
- **Complex Types**: Store types that contain pointers (Vec, String, etc.) which aren't normally storable
- **Type Flexibility**: Store any type implementing `AbiEncode`/`AbiDecode`

## Installation

### Sway Library

Add to your `Forc.toml`:

```toml
[dependencies]
storage_encoded = "0.1.1"
```

### Rust Decoder

Add to your `Cargo.toml`:

```toml
[dependencies]
storage_encoded = "0.1.1"
fuel-core-client = "0.46.0"
fuels = "0.74.0"
```

## Usage

### Sway Contract

```sway
contract;

use storage_encoded::*;

storage {
    data in 0x0000000000000000000000000000000000000000000000000000000000000000: StorageEncoded = StorageEncoded {},
}

struct MyData {
    count: u256,
    items: Vec<u64>,
    name: String,
}

impl Contract {
    #[storage(read, write)]
    fn store_data() {
        let data = MyData {
            count: 42,
            items: vec![1, 2, 3],
            name: String::from_ascii_str("example"),
        };
        storage.data.set::<MyData>(data);
    }

    #[storage(read)]
    fn get_data() -> MyData {
        storage.data.get::<MyData>()
    }
}
```

### Rust Off-chain Reader

```rust
use std::sync::Arc;
use fuel_core_client::client::FuelClient;
use fuel_types::{Bytes32, ContractId};
use fuels::types::{param_types::ParamType, Token};
use storage_encoded::decode_from_storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(FuelClient::new("https://testnet.fuel.network")?);
    let contract_id: ContractId = "0x...".parse()?;
    
    // Define the type structure
    let param_type = ParamType::Struct {
        name: "MyData".into(),
        fields: vec![
            ("count".into(), ParamType::U256),
            ("items".into(), ParamType::Vector(Box::new(ParamType::U64))),
            ("name".into(), ParamType::String),
        ],
        generics: vec![],
    };
    
    // Read directly from storage slot
    let storage_slot: Bytes32 = "0x0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap(); // Your storage slot
    let data = decode_from_storage(client, contract_id, &param_type, &storage_slot).await?;
    
    println!("Data: {:?}", data);
    Ok(())
}
```

## How It Works

### Storage Layout

For a given `StorageEncoded` slot `S`

1. **Length Prefix**: First 8 bytes of the `S` slot store the encoded data length (u64, big-endian)
2. **Data Slots**: Consecutive slots starting at `SHA256(S)` store the encoded data in 32-byte chunks

### Encoding Process

1. **On Write** (`set()`):
   - Data is ABI-encoded into bytes
   - Length is stored in the first 8 bytes of the slot
   - Data is distributed across multiple storage slots

2. **On Read** (`get()`):
   - Length is read from the first slot
   - Data is collected from calculated storage slots
   - Bytes are ABI-decoded back to the original type

### Off-chain Reading

The Rust decoder replicates the storage layout logic to read and decode data directly from the blockchain state without executing any contract code.

## Important Limitations

### ⚠️ Type Safety

**The type system does not enforce encoding/decoding type consistency.** You must use the exact same type for both `set()` and `get()` operations. Mismatched types will cause runtime failures.

```sway
// ❌ WRONG - Will fail at runtime
storage.data.set::<u64>(42);
let value = storage.data.get::<String>(); // Runtime error!

// ✅ CORRECT
storage.data.set::<u64>(42);
let value = storage.data.get::<u64>(); // Works!
```

### Size Limitations

- Maximum encodable size: 2^64 bytes (practically unlimited for most use cases)
- Data must implement `AbiEncode` and `AbiDecode` traits

### Performance Considerations

- **Faster than getters**: Direct reads avoid FuelVM execution overhead
- **Encoding cost**: Both storage and retrieval incur encoding/decoding costs
- **Storage cost**: Encoded data typically uses more storage than raw types

## Testing Status

⚠️ **Experimental**: This library works but has not been thoroughly tested in production environments. Use with caution and test extensively before deploying to mainnet.
