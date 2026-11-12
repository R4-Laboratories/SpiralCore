mod payload;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    let body = match args.as_slice() {
        [] => payload::expected_body(),
        [arg] if arg == "--pair-alpha" => payload::paired_body(1),
        [arg] if arg == "--pair-beta" => payload::paired_body(2),
        _ => return Err("unknown bounded guest fixture".into()),
    };
    let message = mvm_core::mailbox::frame::OutboxMessage {
        recipient: mvm_contract::protocol::mailbox::PrincipalId::from_bytes(
            mvm_core::mailbox::frame::OWN_MAILBOX_RECIPIENT,
        ),
        body,
    };
    mvm_agentd::mailbox_outbox::write_outbox(
        std::path::Path::new(mvm_agentd::mailbox_outbox::OUTBOX_PATH),
        &message,
    )?;
    // Keep the normal admitted workload alive until the host ends the witness.
    std::thread::sleep(std::time::Duration::from_secs(180));
    Ok(())
}
