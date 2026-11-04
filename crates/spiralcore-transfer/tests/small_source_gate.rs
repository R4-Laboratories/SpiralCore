use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
    wan, Error,
};
#[test]
fn small_source_gate_refuses_before_filesystem_or_codec_setup_when_unqualified() {
    let qualified = spiralcore_transfer::zstd_whole::first_block_probe_available();
    println!("source_qualified={qualified}");
    if std::env::var_os("SPIRALCORE_EXPECT_UNQUALIFIED").is_some() {
        assert!(
            !qualified,
            "negative gate did not actually disable qualification"
        );
    }
    if qualified {
        assert!(ResourceProfile::SmallV1.operation_charge().is_ok());
        return;
    }
    assert_eq!(
        ResourceProfile::SmallV1.operation_charge(),
        Err(Error::Unsupported)
    );
    let profile = Capabilities::small_v1()
        .negotiate(&Capabilities::small_v1())
        .unwrap();
    let path =
        std::env::temp_dir().join(format!("spiral-unqualified-small-{}", std::process::id()));
    assert!(!path.exists());
    assert!(matches!(
        ObjectEndpoint::open_with_resource_profile(
            &path,
            ResourceProfile::SmallV1.receiver_limits(),
            wan::BenchAuthority,
            profile,
            ResourceProfile::SmallV1
        ),
        Err(Error::Unsupported)
    ));
    assert!(!path.exists());
    assert!(matches!(
        ObjectSender::new_with_resource_profile(
            ObjectMode::Whole,
            &wan::context([180; 32]),
            None,
            profile,
            ResourceProfile::SmallV1
        ),
        Err(Error::Unsupported)
    ));
    assert!(
        spiralcore_transfer::zstd_whole::PreparedEncoder::new().is_ok(),
        "General remains available by explicit request"
    );
}
