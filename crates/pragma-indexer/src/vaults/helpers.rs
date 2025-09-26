use starknet::core::types::Felt;

pub(crate) fn felt_to_hex_str(felt: Felt) -> String {
    format!("{felt:#64x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_felt_to_hex_str() {
        // Example taken from here: https://voyager.online/event/2091762_1_3
        let felt = Felt::from_dec_str(
            "2271107541705199146619972428603499348671360442682316369626005515476010301932",
        )
        .unwrap();
        assert_eq!(
            felt_to_hex_str(felt),
            "0x50566bca02aef6f3d75364bb03ecd7249292ab65c20c4f9f15506d8578479ec"
        );
    }
}
