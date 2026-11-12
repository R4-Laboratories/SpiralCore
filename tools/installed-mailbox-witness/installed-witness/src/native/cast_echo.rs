/// The only successful workload response is the complete original Cast followed
/// by the exact independent call challenge. No prefix or caller echo suffices.
pub(super) fn verify<'a>(expected: &[u8], challenge: &[u8], stdout: &'a [u8]) -> Option<&'a [u8]> {
    if expected.is_empty()
        || challenge.is_empty()
        || stdout.len() != expected.len().checked_add(challenge.len())?
    {
        return None;
    }
    let (body, echo) = stdout.split_at(expected.len());
    (body == expected && echo == challenge).then_some(body)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_cast_and_challenge_pass() {
        let (body, nonce) = (b"\0A\xffB\0", b"fresh");
        let stdout = [body.as_slice(), nonce.as_slice()].concat();
        assert_eq!(verify(body, nonce, &stdout), Some(body.as_slice()));
        for wrong in [
            body.as_slice(),
            nonce.as_slice(),
            &stdout[..stdout.len() - 1],
            b"\0A\xffB\0stale",
            b"\0A\xffB\0fresh!",
            b"\0A\nB\0fresh",
        ] {
            assert!(verify(body, nonce, wrong).is_none());
        }
    }
}
