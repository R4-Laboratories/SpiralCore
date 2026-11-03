use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    time::Duration,
};
fn main() {
    for (name, after_fin, explicit_halves) in [
        ("open", false, false),
        ("peer-fin", true, false),
        ("peer-fin-explicit-halves", true, true),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut socket, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        println!("case={name}; after_fin={after_fin}; explicit_halves={explicit_halves}");
        if after_fin {
            println!("peer.shutdown(Write)={:?}", peer.shutdown(Shutdown::Write));
            println!("socket.read={:?}", socket.read(&mut [0]));
        }
        println!(
            "socket.shutdown(Both)={:?}",
            socket.shutdown(Shutdown::Both)
        );
        if explicit_halves {
            println!(
                "socket.shutdown(Write)={:?}",
                socket.shutdown(Shutdown::Write)
            );
            println!(
                "socket.shutdown(Read)={:?}",
                socket.shutdown(Shutdown::Read)
            );
        }
        println!("socket.write={:?}", socket.write_all(b"x"));
        let mut received = [0];
        println!(
            "peer.read={:?}; bytes={received:?}",
            peer.read(&mut received)
        );
    }
}
