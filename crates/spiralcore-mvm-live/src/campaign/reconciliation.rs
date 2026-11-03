#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::TrySendError;

    #[test]
    fn retry_commands_have_one_slot_and_drop_disconnects_the_current_signal() {
        let (signal, requests) = bounded_requests();
        assert!(!requests.take_request());
        assert_eq!(signal.request_retry(), Ok(()));
        assert_eq!(signal.request_retry(), Err(TrySendError::Full(())));
        assert!(requests.take_request());
        assert!(!requests.take_request());
        assert_eq!(signal.request_retry(), Ok(()));
        drop(requests);
        assert_eq!(signal.request_retry(), Err(TrySendError::Disconnected(())));
    }

    #[test]
    fn stale_signal_cannot_enqueue_on_the_next_calls_channel() {
        let (old, old_requests) = bounded_requests();
        assert_eq!(old.request_retry(), Ok(()));
        drop(old_requests);
        let (current, requests) = bounded_requests();
        assert_eq!(old.request_retry(), Err(TrySendError::Disconnected(())));
        assert!(!requests.take_request());
        assert_eq!(current.request_retry(), Ok(()));
        assert!(requests.take_request());
        drop(current);
        assert!(!requests.take_request());
    }
}
