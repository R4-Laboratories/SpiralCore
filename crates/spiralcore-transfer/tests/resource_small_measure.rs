#[path = "support/resource_measure.rs"]
mod measurement;
use spiralcore_transfer::{
    resource_profile::ResourceProfile,
    session::SelectionPolicy,
    shared_transfer::PreparedPrediction,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
    wan,
};
#[test]
fn measure_small_profile_with_exact_readbacks() {
    let resources = ResourceProfile::SmallV1;
    measurement::run(
        "small-v1",
        |mode, c, artifact, checkpoint, recipe| {
            let resources = ResourceProfile::SmallV1;
            let profile = Capabilities::small_v1().negotiate(&Capabilities::small_v1())?;
            match mode {
                ObjectMode::Runtime => ObjectSender::with_runtime_resource_profile(
                    c,
                    PreparedPrediction::new_with_resource_profile(checkpoint, recipe, resources)?,
                    profile,
                    resources,
                ),
                ObjectMode::Combined => ObjectSender::with_combined_resource_profile(
                    c,
                    PreparedPrediction::new_with_resource_profile(checkpoint, recipe, resources)?,
                    artifact,
                    SelectionPolicy::Exhaustive,
                    profile,
                    resources,
                ),
                _ => ObjectSender::new_with_resource_profile(
                    mode,
                    c,
                    (mode == ObjectMode::Warm).then_some(artifact),
                    profile,
                    resources,
                ),
            }
        },
        |path| {
            ObjectEndpoint::open_with_resource_profile(
                path,
                ResourceProfile::SmallV1.receiver_limits(),
                wan::BenchAuthority,
                Capabilities::small_v1().negotiate(&Capabilities::small_v1())?,
                ResourceProfile::SmallV1,
            )
        },
        Some(resources.operation_charge().unwrap()),
    );
}
