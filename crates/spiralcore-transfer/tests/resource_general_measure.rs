#[path = "support/resource_measure.rs"]
mod measurement;
use spiralcore_transfer::{
    endpoint::{ColdGraphPolicy, Context},
    session::SelectionPolicy,
    shared_runtime::Recipe,
    shared_transfer::PreparedPrediction,
    v1::{Capabilities, ObjectEndpoint, ObjectMode, ObjectSender},
    wan, Result,
};
fn make_sender(
    mode: ObjectMode,
    c: &Context,
    artifact: &[u8],
    checkpoint: &[u8],
    recipe: &Recipe,
) -> Result<ObjectSender> {
    match mode {
        ObjectMode::Runtime => {
            ObjectSender::with_runtime(c, PreparedPrediction::new(checkpoint, recipe)?)
        }
        ObjectMode::Combined => ObjectSender::with_combined_policy(
            c,
            PreparedPrediction::new(checkpoint, recipe)?,
            artifact,
            SelectionPolicy::Exhaustive,
        ),
        _ => ObjectSender::new(mode, c, (mode == ObjectMode::Warm).then_some(artifact)),
    }
}
#[test]
fn measure_general_profile_with_exact_readbacks() {
    measurement::run(
        "general",
        make_sender,
        |path| {
            ObjectEndpoint::open(
                path,
                wan::limits(),
                wan::BenchAuthority,
                Capabilities::full().negotiate(&Capabilities::full())?,
            )
        },
        None,
    );
}

#[test]
fn measure_general_no_graph_with_exact_readbacks() {
    measurement::run(
        "general-no-graph",
        make_sender,
        |path| {
            ObjectEndpoint::open_with_policy(
                path,
                wan::limits(),
                wan::BenchAuthority,
                Capabilities::full().negotiate(&Capabilities::full())?,
                ColdGraphPolicy::Disabled,
            )
        },
        None,
    );
}
