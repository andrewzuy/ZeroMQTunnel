use std::fs::File;
use std::mem;
use std::net::Ipv4Addr;
use std::os::fd::AsRawFd;

use anyhow::{Context, Result};
use libc::{ifreq, sockaddr_in};
use log::{error, info};
use nix::unistd::{read as nix_read, write as nix_write};

const TUNSETMTU: u64 = 0x400454D3;

pub struct TunDevice {
    file: File,
    name: String,
}

impl TunDevice {
    pub fn new(name: &str, ip: &str, prefix_len: u8, mtu: u32) -> Result<Self> {
        let file = open_tun(name, mtu)?;
        configure_interface(name, ip, prefix_len)?;
        info!("TUN device '{}' configured with {}", name, ip);

        Ok(Self {
            file,
            name: name.to_string(),
        })
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn read_packet(&self, buf: &mut [u8]) -> Result<usize> {
        nix_read(self.file.as_raw_fd(), buf)
            .context("failed to read from TUN")
    }

    pub fn write_packet(&self, buf: &[u8]) -> Result<()> {
        nix_write(&self.file, buf)
            .context("failed to write to TUN")
            .map(|_| ())
    }
}

fn open_tun(name: &str, mtu: u32) -> Result<File> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/net/tun")
        .context("failed to open /dev/net/tun")?;

    let fd = file.as_raw_fd();

    let mut ifr: ifreq = unsafe { mem::zeroed() };
    let name_bytes = name.as_bytes();
    for (i, &b) in name_bytes.iter().enumerate() {
        ifr.ifr_name[i] = b as i8;
    }
    ifr.ifr_ifru.ifru_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as i16;

    let ret = unsafe { libc::ioctl(fd, libc::TUNSETIFF, &ifr) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        anyhow::bail!("ioctl TUNSETIFF failed: {}", err);
    }

    let ret = unsafe { libc::ioctl(fd, TUNSETMTU, mtu as libc::c_uint) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        error!("ioctl TUNSETMTU failed: {} (continuing anyway)", err);
    }

    info!("Opened TUN device '{}'", name);
    Ok(file)
}

fn configure_interface(name: &str, ip_cidr: &str, prefix_len: u8) -> Result<()> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        anyhow::bail!("socket() failed: {}", std::io::Error::last_os_error());
    }

    let result = (|| -> Result<()> {
        let (ip, prefix) = parse_cidr(ip_cidr, prefix_len)?;

        set_if_addr(sock, name, ip)?;

        let netmask = (0xffff_ffffu32 >> (32 - prefix)).to_be();
        set_if_netmask(sock, name, netmask)?;

        set_if_up(sock, name)?;

        Ok(())
    })();

    unsafe { libc::close(sock) };
    result
}

fn parse_cidr(cidr: &str, default_prefix: u8) -> Result<(u32, u8)> {
    let parts: Vec<&str> = cidr.split('/').collect();
    let ip_str = parts[0];
    let prefix = if parts.len() > 1 {
        parts[1].parse::<u8>().context("invalid prefix length")?
    } else {
        default_prefix
    };

    let ip: Ipv4Addr = ip_str.parse().context("invalid IP address")?;
    let ip_u32 = u32::from(ip);
    Ok((ip_u32.to_be(), prefix))
}

fn fill_ifr_name(ifr: &mut ifreq, name: &str) {
    let name_bytes = name.as_bytes();
    for (i, &b) in name_bytes.iter().enumerate() {
        ifr.ifr_name[i] = b as i8;
    }
}

fn set_if_addr(sock: libc::c_int, name: &str, ip: u32) -> Result<()> {
    let mut ifr: ifreq = unsafe { mem::zeroed() };
    fill_ifr_name(&mut ifr, name);

    let mut addr: sockaddr_in = unsafe { mem::zeroed() };
    addr.sin_family = libc::AF_INET as u16;
    addr.sin_addr.s_addr = ip;

    unsafe {
        std::ptr::copy_nonoverlapping(
            &addr as *const _ as *const u8,
            (&mut ifr.ifr_ifru.ifru_addr) as *mut _ as *mut u8,
            mem::size_of::<sockaddr_in>(),
        );
    }

    let ret = unsafe { libc::ioctl(sock, libc::SIOCSIFADDR, &ifr) };
    if ret < 0 {
        anyhow::bail!(
            "ioctl SIOCSIFADDR failed: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}

fn set_if_netmask(sock: libc::c_int, name: &str, netmask: u32) -> Result<()> {
    let mut ifr: ifreq = unsafe { mem::zeroed() };
    fill_ifr_name(&mut ifr, name);

    let mut mask: sockaddr_in = unsafe { mem::zeroed() };
    mask.sin_family = libc::AF_INET as u16;
    mask.sin_addr.s_addr = netmask;

    unsafe {
        std::ptr::copy_nonoverlapping(
            &mask as *const _ as *const u8,
            (&mut ifr.ifr_ifru.ifru_addr) as *mut _ as *mut u8,
            mem::size_of::<sockaddr_in>(),
        );
    }

    let ret = unsafe { libc::ioctl(sock, libc::SIOCSIFNETMASK, &ifr) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        error!("ioctl SIOCSIFNETMASK failed: {} (continuing anyway)", err);
    }
    Ok(())
}

fn set_if_up(sock: libc::c_int, name: &str) -> Result<()> {
    let mut ifr: ifreq = unsafe { mem::zeroed() };
    fill_ifr_name(&mut ifr, name);

    let ret = unsafe { libc::ioctl(sock, libc::SIOCGIFFLAGS, &ifr) };
    if ret < 0 {
        anyhow::bail!(
            "ioctl SIOCGIFFLAGS failed: {}",
            std::io::Error::last_os_error()
        );
    }

    unsafe {
        ifr.ifr_ifru.ifru_flags |= libc::IFF_UP as i16;
    }

    let ret = unsafe { libc::ioctl(sock, libc::SIOCSIFFLAGS, &ifr) };
    if ret < 0 {
        anyhow::bail!(
            "ioctl SIOCSIFFLAGS failed: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(())
}
