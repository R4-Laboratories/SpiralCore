mod payload {
    include!("../../src/payload.rs");
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = mvm_core::mailbox::frame::OutboxMessage {
        recipient: mvm_contract::protocol::mailbox::PrincipalId::from_bytes(
            mvm_core::mailbox::frame::OWN_MAILBOX_RECIPIENT,
        ),
        body: payload::expected_body(),
    };
    mvm_agentd::mailbox_outbox::write_outbox(
        std::path::Path::new(mvm_agentd::mailbox_outbox::OUTBOX_PATH),
        &message,
    )?;
    // Keep the normal admitted workload alive until the host ends the witness.
    std::thread::sleep(std::time::Duration::from_secs(180));
    Ok(())
}
