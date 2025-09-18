use fuel_core_client::client::{FuelClient, types::ContractBalance};
use fuel_types::{AssetId, BlockHeight, Bytes32, ContractId, Word};
use fuel_vm::storage::ContractsStateData;
use futures::TryStreamExt;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ContractStorage {
    contract_id: ContractId,
    slots: HashMap<Bytes32, Option<ContractsStateData>>,
    assets: HashMap<AssetId, Option<Word>>,
}

impl ContractStorage {
    pub fn new(
        contract_id: ContractId,
        slots: Vec<(Bytes32, Vec<u8>)>,
        assets: Vec<ContractBalance>,
    ) -> Self {
        Self {
            contract_id,
            slots: slots
                .into_iter()
                .map(|(k, v)| (k, Some(v.into())))
                .collect(),
            assets: assets
                .into_iter()
                .map(|balance| (balance.asset_id, Some(balance.amount)))
                .collect(),
        }
    }

    pub fn slots(&self) -> &HashMap<Bytes32, Option<ContractsStateData>> {
        &self.slots
    }

    #[allow(dead_code)]
    pub fn assets(&self) -> &HashMap<AssetId, Option<Word>> {
        &self.assets
    }

    #[allow(dead_code)]
    pub async fn slot<'a>(
        &'a mut self,
        key: &Bytes32,
        block_height: &BlockHeight,
        client: &FuelClient,
    ) -> anyhow::Result<Option<&'a ContractsStateData>> {
        if !self.slots.contains_key(key) {
            eprintln!(
                "Fetching asset value from the network for the contract {}.",
                self.contract_id
            );
            let fetched_value = client
                .contract_slots_values(&self.contract_id, Some(*block_height), vec![*key])
                .await?
                .into_iter()
                .next();
            let fetched_value = fetched_value.map(|(_, v)| v.into());
            self.slots.insert(*key, fetched_value);
        }

        Ok(self.slots.get(key).expect("We checked above; qed").as_ref())
    }

    #[allow(dead_code)]
    pub async fn asset(
        &mut self,
        key: &AssetId,
        block_height: &BlockHeight,
        client: &FuelClient,
    ) -> anyhow::Result<Option<Word>> {
        let value = self.assets.get(key);

        match value {
            Some(Some(value)) => Ok(Some(*value)),
            None => {
                eprintln!(
                    "Fetching asset value from the network for the contract {}.",
                    self.contract_id
                );
                let value = client
                    .contract_balance_values(&self.contract_id, Some(*block_height), vec![*key])
                    .await?
                    .into_iter()
                    .next();
                let value = value.map(|v| v.amount);
                self.assets.insert(*key, value.map(|v| v.0));
                Ok(*self.assets.get(key).expect("We inserted value above; qed"))
            }
            Some(None) => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub fn insert_slot(&mut self, key: Bytes32, value: Option<ContractsStateData>) {
        self.slots.insert(key, value);
    }

    #[allow(dead_code)]
    pub fn insert_asset(&mut self, key: AssetId, value: Option<Word>) {
        self.assets.insert(key, value);
    }
}

pub struct ContractStateLoader {
    contract_id: ContractId,
    client: Arc<FuelClient>,
}

impl ContractStateLoader {
    pub fn new(contract_id: ContractId, client: Arc<FuelClient>) -> Self {
        Self {
            contract_id,
            client,
        }
    }

    pub async fn load_contract_state(
        self,
        target_block_height: BlockHeight,
    ) -> anyhow::Result<ContractStorage> {
        let client = self.client.clone();
        let slots_task = tokio::task::spawn(async move {
            let slots_keys = client
                .contract_storage_slots(&self.contract_id)
                .await?
                .map_ok(|(key, _)| key)
                .try_collect::<Vec<_>>()
                .await?;

            let slots_values = client
                .contract_slots_values(&self.contract_id, Some(target_block_height), slots_keys)
                .await?;

            Ok::<_, anyhow::Error>(slots_values)
        });

        let client = self.client.clone();
        let balances_task = tokio::task::spawn(async move {
            let asset_keys = client
                .contract_storage_balances(&self.contract_id)
                .await?
                .map_ok(|balance| AssetId::from(balance.asset_id))
                .try_collect::<Vec<_>>()
                .await?;

            let asset_values = client
                .contract_balance_values(&self.contract_id, Some(target_block_height), asset_keys)
                .await?;

            Ok::<_, anyhow::Error>(asset_values)
        });

        let (slots_result, assets_result) = futures::future::join(slots_task, balances_task).await;
        let (slots, assets) = (slots_result??, assets_result??);

        let contract_storage = ContractStorage::new(
            self.contract_id,
            slots,
            assets.into_iter().map(Into::into).collect(),
        );

        Ok(contract_storage)
    }
}
