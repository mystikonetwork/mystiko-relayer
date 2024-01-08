pub mod handler;
mod request;
mod response;

use crate::service::v1::request::{TransactRequestV1, TransactionTypeV1};
use anyhow::Result;
use ethereum_types::U256;
use ethers_core::types::Bytes;
use log::debug;
use mystiko_abi::commitment_pool::{G1Point, G2Point, Proof, TransactRequest};
use mystiko_protos::core::v1::SpendType;
use mystiko_relayer_types::TransactRequestData;

pub fn parse_transact_request(request: TransactRequestV1, asset_decimals: u32) -> Result<TransactRequestData> {
    let sig_pk = convert_sig_pk(request.sig_pk)?;
    let out_encrypted_notes = convert_out_encrypted_notes(request.out_encrypted_notes)?;
    let random_auditing_public_key = convert_random_auditing_public_key(request.random_auditing_public_key.as_str())?;
    let encrypted_auditor_notes = convert_encrypted_auditor_notes(request.encrypted_auditor_notes)?;
    Ok(TransactRequestData {
        contract_param: TransactRequest {
            proof: Proof {
                a: G1Point {
                    x: request.proof.a.x,
                    y: request.proof.a.y,
                },
                b: G2Point {
                    x: request.proof.b.x,
                    y: request.proof.b.y,
                },
                c: G1Point {
                    x: request.proof.c.x,
                    y: request.proof.c.y,
                },
            },
            root_hash: request.root_hash,
            serial_numbers: request.serial_numbers,
            sig_hashes: request.sig_hashes,
            sig_pk,
            public_amount: request.public_amount,
            relayer_fee_amount: request.relayer_fee_amount,
            out_commitments: request.out_commitments,
            out_rollup_fees: request.out_rollup_fees,
            public_recipient: request.public_recipient,
            relayer_address: request.relayer_address,
            out_encrypted_notes,
            random_auditing_public_key,
            encrypted_auditor_notes,
        },
        spend_type: convert_spend_type(request.transaction_type),
        bridge_type: request.bridge_type,
        chain_id: request.chain_id,
        asset_symbol: request.asset_symbol,
        asset_decimals,
        pool_address: request.pool_address,
        circuit_type: request.circuit_type,
        signature: request.signature,
    })
}

fn convert_sig_pk(sig_pk: String) -> Result<[u8; 32]> {
    let decode = hex::decode(&sig_pk[2..])?;
    let mut result = [0u8; 32];
    result.copy_from_slice(decode.as_slice());
    Ok(result)
}

fn convert_out_encrypted_notes(out_encrypted_notes: Vec<String>) -> Result<Vec<Bytes>> {
    let mut result: Vec<Bytes> = vec![];
    for notes in out_encrypted_notes {
        let decode = hex::decode(&notes[2..])?;
        let bytes: Bytes = Bytes::from(decode);
        result.push(bytes);
    }
    Ok(result)
}

fn convert_encrypted_auditor_notes(out_encrypted_notes: Vec<String>) -> Result<Vec<U256>> {
    let mut result: Vec<U256> = vec![];
    for notes in &out_encrypted_notes {
        result.push(U256::from_dec_str(notes)?);
    }
    debug!("convert encrypted auditor notes {:?}", result);
    Ok(result)
}

fn convert_random_auditing_public_key(key: &str) -> Result<U256> {
    let result = U256::from_dec_str(key)?;
    debug!("convert random auditing public key {:?}", result);
    Ok(result)
}

fn convert_spend_type(t: TransactionTypeV1) -> SpendType {
    match t {
        TransactionTypeV1::Transfer => SpendType::Transfer,
        TransactionTypeV1::Withdraw => SpendType::Withdraw,
    }
}
