use std::sync::Arc;

use anyhow::anyhow;
use fuel_core_client::client::FuelClient;
use fuel_types::{Bytes32, ContractId};
use fuels::{
    core::codec::ABIDecoder,
    types::{Token, param_types::ParamType},
};
use local_tx_executor::contract_state_loader::ContractStateLoader;
use sha2::{Digest, Sha256};

pub async fn decode_from_storage(
    client: Arc<FuelClient>,
    contract_id: ContractId,
    param_type: &ParamType,
    slot: &Bytes32,
) -> Result<Token, anyhow::Error> {
    let val: Bytes32 = "0x1f".parse().unwrap();
    let block_height = client
        .chain_info()
        .await
        .unwrap()
        .latest_block
        .header
        .height;
    let contract_state_loader = ContractStateLoader::new(contract_id, client);
    let contract_storage = contract_state_loader
        .load_contract_state(block_height.into())
        .await?;

    let slots = contract_storage.slots();

    // fetch data length
    let len = &slots
        .get(slot)
        .and_then(Option::as_ref)
        .ok_or(anyhow!("No length for encoded bytes"))?
        .0[0..8];
    let len = u64::from_be_bytes([
        len[0], len[1], len[2], len[3], len[4], len[5], len[6], len[7],
    ]);

    // calculate data slot
    let mut hasher = Sha256::new();
    hasher.update(slot.as_slice());
    let first_data_slot = hasher.finalize();
    let first_data_slot = Bytes32::from_bytes_ref_checked(&first_data_slot[..]).unwrap();

    // read data
    let mut buf = Vec::with_capacity(len.try_into().unwrap_or_default());
    for offset in 0..(len / 32) {
        let data_slot = add_offset(first_data_slot, offset);
        let mut data = slots
            .get(&data_slot)
            .and_then(Option::as_ref)
            .ok_or(anyhow!("Missing data slot"))?
            .0
            .clone();
        buf.append(&mut data);
    }
    let remainder_data_slot = add_offset(first_data_slot, len / 32);
    let remainder_data = slots
        .get(&remainder_data_slot)
        .and_then(Option::as_ref)
        .map(|d| d.0.clone())
        .unwrap_or(vec![0u8; 32]);

    let mut i = 0;
    while buf.len() < len.try_into().unwrap() {
        buf.push(remainder_data[i]);
        i += 1;
    }

    // decode data
    let decoder = ABIDecoder::default();
    Ok(decoder.decode(param_type, buf.as_slice())?)
}

fn add_offset(slot: &Bytes32, offset: u64) -> Bytes32 {
    let tail = &slot.as_slice()[24..32];
    let tail = u64::from_be_bytes([
        tail[0], tail[1], tail[2], tail[3], tail[4], tail[5], tail[6], tail[7],
    ]);

    let new_tail = (tail + offset).to_be_bytes(); // FIXME: handle overflow on max value

    let mut res = *slot;
    res.as_mut_slice()[24..32].copy_from_slice(&new_tail);
    res
}

// fn storage_vec_slot(original_slot: &Bytes32) -> Bytes32 {}

#[tokio::test]
async fn test_decode_from_storage() {
    let client = Arc::new(FuelClient::new("https://testnet.fuel.network").unwrap());
    let contract_id: ContractId =
        "0xb6cedf9dd9ee152d7ed46e5cb081f5cc4f0ec57fe60fb411f1171b881a96201b"
            .parse()
            .unwrap();

    let param_type = ParamType::Struct {
        name: "Foobar".into(),
        fields: vec![
            ("a".into(), ParamType::U256),
            ("b".into(), ParamType::U64),
            ("c".into(), ParamType::U8),
        ],
        generics: vec![],
    };
    let res = decode_from_storage(client, contract_id, &param_type, &Bytes32::zeroed())
        .await
        .unwrap();
    assert_eq!(
        res,
        Token::Struct(vec![
            Token::U256(fuels::types::U256([42, 0, 0, 0])),
            Token::U64(1337),
            Token::U8(77),
        ],)
    )
}
