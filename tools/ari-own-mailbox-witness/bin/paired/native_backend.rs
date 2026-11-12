//! Native backends admitted by this paired qualification driver.
pub fn select(os: &str, arch: &str, requested: Option<&str>) -> Result<&'static str, &'static str> {
    let native = match (os, arch) {
        ("linux", "x86_64" | "aarch64") => "firecracker",
        ("macos", "aarch64") => "hvf",
        _ => return Err("paired native qualification requires Linux/KVM or Apple Silicon/HVF"),
    };
    if requested.is_some_and(|name| name != native) {
        return Err("requested backend does not match the native host role");
    }
    Ok(native)
}

pub fn check_observation(requested: &str, actual: Option<&str>) -> Result<(), &'static str> {
    match actual {
        Some(backend) if requested == backend => Ok(()),
        Some(_) => Err("original VM backend marker differs from the requested native backend"),
        None => Err("original VM has no observed backend marker"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_only_native_roles() {
        for (os, arch, backend) in [
            ("macos", "aarch64", "hvf"),
            ("linux", "x86_64", "firecracker"),
            ("linux", "aarch64", "firecracker"),
        ] {
            assert_eq!(select(os, arch, None), Ok(backend));
            assert_eq!(select(os, arch, Some(backend)), Ok(backend));
            for other in ["qemu", "mock", "libkrun", "apple-container", "auto", ""] {
                assert!(select(os, arch, Some(other)).is_err());
            }
        }
        assert!(select("macos", "aarch64", Some("firecracker")).is_err());
        assert!(select("linux", "aarch64", Some("hvf")).is_err());
        assert!(select("macos", "x86_64", None).is_err());
        assert!(select("windows", "aarch64", None).is_err());
    }

    #[test]
    fn accepts_only_a_matching_original_marker() {
        assert_eq!(check_observation("hvf", Some("hvf")), Ok(()));
        assert_eq!(
            check_observation("firecracker", Some("firecracker")),
            Ok(())
        );
        assert!(check_observation("hvf", None).is_err());
        assert!(check_observation("firecracker", Some("hvf")).is_err());
        assert!(check_observation("hvf", Some("libkrun")).is_err());
    }
}
