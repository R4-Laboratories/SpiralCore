fn main() {
    let backend = mvm_runtime::AnyBackend::from_hypervisor("firecracker");
    assert_eq!(backend.name(), "firecracker");
    println!("backend={} original_lifetime={}", backend.name(), backend.supports_host_lifetime());
    assert!(backend.supports_host_lifetime(), "canonical Client preflight must refuse this backend");
}
