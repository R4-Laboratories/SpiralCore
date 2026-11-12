//! Evidence for the explicitly qualified lost-first-receipt run.
#[derive(Debug)]
pub struct FirstReceipt<T> {
    envelope: T,
    pub bytes: Vec<u8>,
    pub forward_bytes: usize,
    pub at_ns: u64,
}

impl<T: Eq> FirstReceipt<T> {
    pub fn lost(
        envelope: T,
        bytes: Vec<u8>,
        forward_bytes: usize,
        returned_bytes: usize,
        at_ns: u64,
    ) -> Result<Self, &'static str> {
        if returned_bytes != 0 || bytes.is_empty() || forward_bytes == 0 {
            return Err("first original B receipt was absent or already reached A");
        }
        Ok(Self {
            envelope,
            bytes,
            forward_bytes,
            at_ns,
        })
    }

    pub fn verify_retry(
        &self,
        envelope: &T,
        receipt: &[u8],
        forward: usize,
        returned: usize,
    ) -> Result<(), &'static str> {
        if self.envelope != *envelope
            || self.bytes.as_slice() != receipt
            || self.forward_bytes != forward
            || returned != receipt.len()
        {
            return Err("retry changed the original envelope, signed receipt, or byte counts");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FirstReceipt;

    #[test]
    fn first_return_must_be_lost_and_nonempty() {
        assert!(FirstReceipt::lost(8u8, vec![1, 2], 7, 2, 4).is_err());
        assert!(FirstReceipt::lost(8u8, Vec::new(), 7, 0, 4).is_err());
    }

    #[test]
    fn retry_requires_the_same_envelope_signed_bytes_and_full_return() {
        let first = FirstReceipt::lost(8u8, vec![1, 2], 7, 0, 4).unwrap();
        assert_eq!(first.at_ns, 4);
        assert!(first.verify_retry(&8, &[1, 2], 7, 2).is_ok());
        assert!(first.verify_retry(&9, &[1, 2], 7, 2).is_err());
        assert!(first.verify_retry(&8, &[1, 3], 7, 2).is_err());
        assert!(first.verify_retry(&8, &[1, 2], 8, 2).is_err());
        assert!(first.verify_retry(&8, &[1, 2], 7, 0).is_err());
    }
}
