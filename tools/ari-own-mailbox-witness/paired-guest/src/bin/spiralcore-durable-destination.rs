fn main() -> std::io::Result<()> {
    spiralcore_paired_journal_workloads::serve(
        spiralcore_paired_journal_workloads::Role::Destination,
    )
}
