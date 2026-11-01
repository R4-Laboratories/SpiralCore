#![cfg(target_os = "linux")]

use spiralcore_gateway::tun::Tun;

#[test]
fn invalid_interface_names_fail_before_opening_the_tun_device() {
    for name in ["", "abcdefghijklmnop", "bad/name", "bad\0name", "bad name"] {
        let error = match Tun::open(name) {
            Ok(_) => panic!("invalid name accepted"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }
}

#[test]
fn an_owned_non_tun_descriptor_is_rejected() {
    let (socket, _other) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(Tun::from_owned_fd(socket.into()).is_err());
}
