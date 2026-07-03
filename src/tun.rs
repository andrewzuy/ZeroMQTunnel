use std::fs::File;
use std::mem;
use std::net::Ipv4Addr;
use std::os::fd::AsRawFd;

use anyhow::{Context, Result};
use libc::ifreq;
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
    let (ip, prefix) = parse_cidr(ip_cidr, prefix_len)?;
    let cidr = format!("{}/{}", ip, prefix);

    run_cmd(&["ip", "addr", "add", &cidr, "dev", name])?;
    run_cmd(&["ip", "link", "set", "up", name])?;

    info!("Configured interface {} with {}", name, cidr);
    Ok(())
}

fn run_cmd(args: &[&str]) -> Result<()> {
    let output = std::process::Command::new(args[0])
        .args(&args[1..])
        .output()
        .context(format!("failed to execute {:?}", args[0]))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "{:?} failed: {}",
            args,
            stderr.trim()
        );
    }
    Ok(())
}

fn parse_cidr(cidr: &str, default_prefix: u8) -> Result<(String, u8)> {
    let parts: Vec<&str> = cidr.split('/').collect();
    let ip_str = parts[0].to_string();
    let prefix = if parts.len() > 1 {
        parts[1].parse::<u8>().context("invalid prefix length")?
    } else {
        default_prefix
    };

    let _: Ipv4Addr = ip_str.parse().context("invalid IP address")?;
    Ok((ip_str, prefix))
}
