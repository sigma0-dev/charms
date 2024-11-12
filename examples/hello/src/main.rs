use ark_serialize::CanonicalSerialize;
use charms_data::{AppId, Charm, Data, Transaction, Utxo, UtxoId, VkHash, TOKEN};
use jolt::{Jolt, JoltCommitments, RV32IJoltProof, RV32IJoltVM, F, PCS};

pub fn main() {
    let (program, prep) = guest::preprocess_zk_meme_token_policy();

    let token_app_id = AppId {
        tag: TOKEN.to_vec(),
        id: UtxoId::default(),
        vk_hash: VkHash::default(),
    };

    let tx = Transaction {
        ins: vec![Utxo {
            id: Some(UtxoId::default()),
            charm: Charm::from([(token_app_id.clone(), 1u64.into())]),
        }],
        refs: vec![],
        outs: vec![Utxo {
            id: None,
            charm: Charm::from([(token_app_id.clone(), 1u64.into())]),
        }],
    };

    let (output, proof) = guest::prove_zk_meme_token_policy(
        program,
        prep.clone(),
        token_app_id,
        tx,
        Data::empty(),
        Data::empty(),
    );

    // let RV32IJoltProof::<F, PCS> {
    //     trace_length,
    //     program_io,
    //     bytecode,
    //     read_write_memory,
    //     instruction_lookups,
    //     r1cs,
    //     opening_proof,
    // } = proof.proof;
    //
    // let commitments = proof.commitments;
    //
    // let mut bytecode_buf = vec![];
    // bytecode.serialize_compressed(&mut bytecode_buf).unwrap();
    //
    // let mut rw_mem_buf = vec![];
    // read_write_memory
    //     .serialize_compressed(&mut rw_mem_buf)
    //     .unwrap();
    //
    // let mut instr_lookups_buf = vec![];
    // instruction_lookups
    //     .serialize_compressed(&mut instr_lookups_buf)
    //     .unwrap();
    //
    // let mut r1cs_buf = vec![];
    // r1cs.serialize_compressed(&mut r1cs_buf).unwrap();
    //
    // let mut opening_proof_buf = vec![];
    // opening_proof
    //     .serialize_compressed(&mut opening_proof_buf)
    //     .unwrap();
    //
    // let mut commitments_buf = vec![];
    // commitments
    //     .serialize_compressed(&mut commitments_buf)
    //     .unwrap();
    //
    // dbg!(trace_length);
    // dbg!(bytecode_buf.len());
    // dbg!(rw_mem_buf.len());
    // dbg!(instr_lookups_buf.len());
    // dbg!(r1cs_buf.len());
    // dbg!(opening_proof_buf.len());
    // dbg!(commitments_buf.len());
    //
    // let total = bytecode_buf.len()
    //     + rw_mem_buf.len()
    //     + instr_lookups_buf.len()
    //     + r1cs_buf.len()
    //     + opening_proof_buf.len()
    //     + commitments_buf.len();
    // dbg!(total);

    // let mut buf = vec![];
    // proof.proof.serialize_compressed(&mut buf).unwrap();

    let is_valid = RV32IJoltVM::verify(prep, proof.proof, proof.commitments, None).is_ok();

    dbg!(output);
    dbg!(is_valid);
}
