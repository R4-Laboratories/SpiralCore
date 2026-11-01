//! Linux packet I/O only. Interface addresses, routes, and MTU are provisioned externally.
use std::{
    fs::OpenOptions,
    io,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::OpenOptionsExt,
    },
};
use tokio::io::unix::AsyncFd;

pub const MTU: usize = 1280;

pub struct Tun {
    fd: AsyncFd<OwnedFd>,
    name: String,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn request(name: &str) -> io::Result<libc::ifreq> {
    if name.is_empty()
        || name.len() >= libc::IFNAMSIZ
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b))
    {
        return Err(invalid("invalid interface name"));
    }
    // SAFETY: ifreq is a plain C request; its all-zero state is valid.
    let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
    for (dst, byte) in request.ifr_name.iter_mut().zip(name.bytes()) {
        *dst = byte as libc::c_char;
    }
    Ok(request)
}

fn check_mtu(name: &str) -> io::Result<()> {
    // SAFETY: socket has no pointer arguments; a successful descriptor is owned below.
    let raw = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 0) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: raw is a newly returned descriptor, now exclusively owned by this value.
    let socket = unsafe { OwnedFd::from_raw_fd(raw) };
    let mut request = request(name)?;
    // SAFETY: request points to a writable ifreq for the lifetime of the call.
    if unsafe { libc::ioctl(socket.as_raw_fd(), libc::SIOCGIFMTU as _, &mut request) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful SIOCGIFMTU initialized the ifru_mtu union member.
    if unsafe { request.ifr_ifru.ifru_mtu } != MTU as libc::c_int {
        return Err(invalid("TUN interface must already have MTU 1280"));
    }
    Ok(())
}

impl Tun {
    pub fn open(name: &str) -> io::Result<Self> {
        let mut request = request(name)?;
        check_mtu(name)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/net/tun")?;
        request.ifr_ifru.ifru_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as libc::c_short;
        // SAFETY: the live TUN descriptor and writable ifreq are valid throughout ioctl.
        if unsafe { libc::ioctl(file.as_raw_fd(), libc::TUNSETIFF, &mut request) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Self::from_owned_fd(file.into())
    }

    pub fn from_owned_fd(fd: OwnedFd) -> io::Result<Self> {
        // SAFETY: ifreq is plain C storage with a valid all-zero initial state.
        let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
        // SAFETY: fd is owned and request is writable for this synchronous ioctl.
        if unsafe { libc::ioctl(fd.as_raw_fd(), libc::TUNGETIFF, &mut request) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: TUNGETIFF initializes the flags member on success.
        let flags = unsafe { request.ifr_ifru.ifru_flags } as libc::c_int;
        let framing = libc::IFF_TUN | libc::IFF_TAP | libc::IFF_NO_PI | libc::IFF_VNET_HDR;
        if flags & framing != (libc::IFF_TUN | libc::IFF_NO_PI) {
            return Err(invalid(
                "plain TUN without packet-info or virtio headers required",
            ));
        }
        let name_bytes: Vec<u8> = request
            .ifr_name
            .iter()
            .map(|&b| b as u8)
            .take_while(|&b| b != 0)
            .collect();
        let name = String::from_utf8(name_bytes).map_err(|_| invalid("non-UTF8 interface name"))?;
        check_mtu(&name)?;
        // SAFETY: fcntl operates on the owned, live descriptor without pointer arguments.
        let status = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
        if status < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: preserve existing descriptor status flags while enabling nonblocking I/O.
        if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, status | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: F_GETFD returns the owned descriptor's flags.
        let descriptor = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: preserve flags and prevent the descriptor from leaking across exec.
        if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, descriptor | libc::FD_CLOEXEC) } < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            // SAFETY: the OwnedFd cannot change its descriptor while registered; only
            // borrowed reads/writes are exposed, and AsyncFd owns it until deregistration.
            fd: unsafe { AsyncFd::register(fd)? },
            name,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn read_packet(&self) -> io::Result<Vec<u8>> {
        // One extra byte detects a packet larger than the fixed MTU, including truncation.
        let mut packet = vec![0; MTU + 1];
        loop {
            let mut ready = self.fd.readable().await?;
            let result = ready.try_io(|inner| {
                // SAFETY: packet is writable for its entire length; descriptor remains owned.
                let count = unsafe {
                    libc::read(
                        inner.get_ref().as_raw_fd(),
                        packet.as_mut_ptr().cast(),
                        packet.len(),
                    )
                };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Err(_) => continue,
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(error)) => return Err(error),
                Ok(Ok(0)) => {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TUN closed"))
                }
                Ok(Ok(count)) if count > MTU => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "TUN packet exceeds MTU",
                    ))
                }
                Ok(Ok(count)) => {
                    packet.truncate(count);
                    return Ok(packet);
                }
            }
        }
    }

    pub async fn write_packet(&self, packet: &[u8]) -> io::Result<()> {
        if packet.is_empty() || packet.len() > MTU {
            return Err(invalid("TUN packet length"));
        }
        loop {
            let mut ready = self.fd.writable().await?;
            let result = ready.try_io(|inner| {
                // SAFETY: packet is readable for its entire length and descriptor remains owned.
                let count = unsafe {
                    libc::write(
                        inner.get_ref().as_raw_fd(),
                        packet.as_ptr().cast(),
                        packet.len(),
                    )
                };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Err(_) => continue,
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(error)) => return Err(error),
                Ok(Ok(count)) if count == packet.len() => return Ok(()),
                Ok(Ok(_)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "short TUN packet write",
                    ))
                }
            }
        }
    }
}
