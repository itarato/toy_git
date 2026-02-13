pub(crate) fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod test {
    use crate::common::bytes_to_string;

    #[test]
    fn test_bytes_to_string() {
        assert_eq!("0314a3", bytes_to_string(&[0x03, 0x14, 0xa3]));
    }
}
