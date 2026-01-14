use alloy::{
    consensus::Header,
    rlp::Encodable as _,
    rpc::types::{debug::ExecutionWitness, Block},
};

#[derive(Clone)]
pub struct EthBlockInput {
    pub transactions: Vec<Vec<u8>>,
    pub execution_witness: ExecutionWitness,
    pub block_header: Header,
    pub withdrawals_rlp: Vec<u8>,
}

impl EthBlockInput {
    pub fn new(block: Block, execution_witness: ExecutionWitness) -> Self {
        let withdrawals_rlp = if let Some(withdrawals) = block.withdrawals.clone() {
            let mut buffer = Vec::new();
            withdrawals.encode(&mut buffer);
            buffer
        } else {
            Vec::new()
        };
        let transactions = block
            .transactions
            .clone()
            .into_transactions()
            .map(|tx| tx.inner.into_encoded().encoded_bytes().to_vec())
            .collect();

        Self {
            transactions,
            execution_witness,
            block_header: block.header.clone().into(),
            withdrawals_rlp,
        }
    }
}
