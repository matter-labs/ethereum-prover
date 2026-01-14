use alloy::consensus::Header;
use alloy::rlp::{Decodable, Encodable};
use anyhow::{anyhow, bail};
use basic_system::system_implementation::ethereum_storage_model::caches::account_properties::EthereumAccountProperties;
use basic_system::system_implementation::ethereum_storage_model::{
    digits_from_key, EthereumMPT, Path as MptPath,
};
use crypto::MiniDigest;
use forward_system::run::query_processors::{
    EthereumCLResponder, EthereumTargetBlockHeaderResponder, GenericPreimageResponder,
    InMemoryEthereumInitialAccountStateResponder, InMemoryEthereumInitialStorageSlotValueResponder,
    TxDataResponder, UARTPrintReponsder,
};
use forward_system::run::test_impl::InMemoryPreimageSource;
use forward_system::run::test_impl::TxListSource;
use oracle_provider::ZkEENonDeterminismSource;
use ruint::aliases::B160;
use std::alloc::Global;
use std::collections::{BTreeMap, HashMap};
use zk_ee::memory::vec_trait::VecCtor;
use zk_ee::utils::Bytes32;

use crate::prover::types::EthBlockInput;

pub(crate) fn build_oracle(input: EthBlockInput) -> anyhow::Result<ZkEENonDeterminismSource> {
    let mut headers: Vec<Header> = input
        .execution_witness
        .headers
        .iter()
        .map(|el| {
            let mut slice: &[u8] = &el.0;
            Header::decode(&mut slice).map_err(|_| anyhow!("failed to decode header"))
        })
        .collect::<anyhow::Result<_>>()?;

    if headers.is_empty() {
        bail!("execution witness contains no headers");
    }
    if !headers.is_sorted_by(|a, b| a.number < b.number) {
        bail!("execution witness headers are not sorted");
    }

    headers.reverse();

    let mut headers_encodings: Vec<_> = input
        .execution_witness
        .headers
        .iter()
        .map(|el| el.0.to_vec())
        .collect();
    headers_encodings.reverse();

    let initial_root = headers[0].state_root;

    let mut preimage_source = InMemoryPreimageSource::default();
    let mut preimages_oracle: BTreeMap<Bytes32, Vec<u8>> = BTreeMap::new();

    for el in input.execution_witness.state.iter() {
        let hash = crypto::sha3::Keccak256::digest(el);
        preimages_oracle.insert(Bytes32::from_array(hash), el.to_vec());
        preimage_source
            .inner
            .insert(Bytes32::from_array(hash), el.to_vec());
    }

    for el in input.execution_witness.codes.iter() {
        let hash = crypto::sha3::Keccak256::digest(el);
        preimages_oracle.insert(Bytes32::from_array(hash), el.to_vec());
        preimage_source
            .inner
            .insert(Bytes32::from_array(hash), el.to_vec());
    }

    let mut interner =
        basic_system::system_implementation::ethereum_storage_model::BoxInterner::with_capacity_in(
            1 << 26,
            Global,
        );
    let mut hasher = crypto::sha3::Keccak256::new();
    let mut accounts_mpt: EthereumMPT<'_, Global, VecCtor> =
        EthereumMPT::new_in(initial_root.0, &mut interner, Global)
            .map_err(|_| anyhow!("failed to initialize accounts MPT"))?;

    let mut account_properties = HashMap::<B160, EthereumAccountProperties>::new();
    for el in input.execution_witness.keys.iter() {
        if el.len() == 20 {
            let hash = crypto::sha3::Keccak256::digest(el);
            let digits = digits_from_key(&hash);
            let path = MptPath::new(&digits);
            if let Ok(props) =
                accounts_mpt.get(path, &mut preimages_oracle, &mut interner, &mut hasher)
            {
                let props = EthereumAccountProperties::parse_from_rlp_bytes(props)
                    .map_err(|_| anyhow!("failed to parse account properties"))?;
                let key = B160::from_be_bytes::<20>(el[..].try_into().unwrap());
                account_properties.insert(key, props);
            }
        }
    }

    let tx_source = TxListSource {
        transactions: input.transactions.into(),
    };

    let mut target_header_encoding = vec![];
    input.block_header.encode(&mut target_header_encoding);

    let target_header_responder = EthereumTargetBlockHeaderResponder {
        target_header: input.block_header,
        target_header_encoding,
    };
    let tx_data_responder = TxDataResponder {
        tx_source,
        next_tx: None,
    };
    let preimage_responder = GenericPreimageResponder { preimage_source };
    let initial_account_state_responder = InMemoryEthereumInitialAccountStateResponder::new(
        initial_root.0,
        account_properties.clone(),
        preimages_oracle.clone(),
    );
    let initial_values_responder =
        InMemoryEthereumInitialStorageSlotValueResponder::new(account_properties, preimages_oracle);

    let cl_responder = EthereumCLResponder {
        withdrawals_list: input.withdrawals_rlp,
        parent_headers_list: headers,
        parent_headers_encodings_list: headers_encodings,
    };

    let mut oracle = ZkEENonDeterminismSource::default();
    oracle.add_external_processor(target_header_responder);
    oracle.add_external_processor(tx_data_responder);
    oracle.add_external_processor(preimage_responder);
    oracle.add_external_processor(initial_account_state_responder);
    oracle.add_external_processor(initial_values_responder);
    oracle.add_external_processor(cl_responder);
    oracle.add_external_processor(UARTPrintReponsder);
    oracle.add_external_processor(callable_oracles::arithmetic::ArithmeticQuery);
    oracle.add_external_processor(callable_oracles::field_hints::FieldOpsQuery);

    Ok(oracle)
}
