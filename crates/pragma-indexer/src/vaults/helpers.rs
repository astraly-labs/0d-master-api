use starknet::core::types::Felt;

pub(crate) fn felt_to_hex_str(felt: Felt) -> String {
    format!("{:#64x}", felt)
}
