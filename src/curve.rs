use std::fs;

use anyhow::{Context, Result};
use log::info;
use zmq::CurveKeyPair;

/// Z85-encoded CURVE key pair for persistence.
#[derive(Clone)]
pub struct CurveKeys {
    pub public_key_z85: String,
    pub secret_key_z85: String,
    pub public_key: [u8; 32],
    pub secret_key: [u8; 32],
}

impl CurveKeys {
    /// Generate a new CURVE key pair.
    pub fn generate() -> Result<Self> {
        let pair = CurveKeyPair::new().context("failed to generate CURVE key pair")?;
        let public_z85 = zmq::z85_encode(&pair.public_key)
            .context("failed to encode public key")?;
        let secret_z85 = zmq::z85_encode(&pair.secret_key)
            .context("failed to encode secret key")?;
        Ok(Self {
            public_key_z85: public_z85,
            secret_key_z85: secret_z85,
            public_key: pair.public_key,
            secret_key: pair.secret_key,
        })
    }

    /// Load keys from Z85-encoded strings.
    pub fn from_z85(public_z85: &str, secret_z85: &str) -> Result<Self> {
        let public_bytes = zmq::z85_decode(public_z85)
            .context("failed to decode public key")?;
        let secret_bytes = zmq::z85_decode(secret_z85)
            .context("failed to decode secret key")?;
        let public_key: [u8; 32] = public_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("public key must be 32 bytes"))?;
        let secret_key: [u8; 32] = secret_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("secret key must be 32 bytes"))?;
        Ok(Self {
            public_key_z85: public_z85.to_string(),
            secret_key_z85: secret_z85.to_string(),
            public_key,
            secret_key,
        })
    }

    /// Save keys to a file as Z85-encoded text (two lines: public, secret).
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = format!("{}\n{}\n", self.public_key_z85, self.secret_key_z85);
        fs::write(path, &content)
            .context(format!("failed to write key file {}", path))?;
        info!("CURVE keys written to {}", path);
        Ok(())
    }

    /// Load keys from a file (two lines: public key Z85, secret key Z85).
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context(format!("failed to read key file {}", path))?;
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 2 {
            anyhow::bail!("key file must contain two lines (public key, secret key)");
        }
        Self::from_z85(lines[0].trim(), lines[1].trim())
    }

    /// Print keys to stdout (useful for keygen subcommand).
    pub fn print(&self) {
        println!("Public key (Z85):  {}", self.public_key_z85);
        println!("Secret key (Z85): {}", self.secret_key_z85);
    }
}

/// Optional CURVE configuration.
#[derive(Clone)]
pub struct CurveConfig {
    pub enabled: bool,
    pub own_keys: CurveKeys,
    pub server_public_key: Option<[u8; 32]>,
}

impl CurveConfig {
    /// Build config for server mode.
    pub fn for_server(enabled: bool, key_file: Option<&str>) -> Result<Option<Self>> {
        if !enabled {
            return Ok(None);
        }
        let keys = if let Some(path) = key_file {
            CurveKeys::load_from_file(path)
                .context(format!("failed to load server keys from {}", path))?
        } else {
            CurveKeys::generate()?
        };
        Ok(Some(Self {
            enabled: true,
            own_keys: keys,
            server_public_key: None,
        }))
    }

    /// Build config for client mode.
    pub fn for_client(
        enabled: bool,
        client_key_file: Option<&str>,
        server_public_key_z85: Option<&str>,
    ) -> Result<Option<Self>> {
        if !enabled {
            return Ok(None);
        }
        let keys = if let Some(path) = client_key_file {
            CurveKeys::load_from_file(path)
                .context(format!("failed to load client keys from {}", path))?
        } else {
            CurveKeys::generate()?
        };
        let server_pk_bytes = if let Some(z85) = server_public_key_z85 {
            zmq::z85_decode(z85)
                .context("failed to decode server public key")?
        } else {
            anyhow::bail!("--server-public-key is required when --enable-curve is set in client mode");
        };
        let server_pk: [u8; 32] = server_pk_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("server public key must be 32 bytes"))?;
        Ok(Some(Self {
            enabled: true,
            own_keys: keys,
            server_public_key: Some(server_pk),
        }))
    }
}
