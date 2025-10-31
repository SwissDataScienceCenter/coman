use std::fmt::{Debug, Display};

use color_eyre::Result;
use keyring::KeyringEntry;

pub struct Secret(pub String);

impl Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<redacted>")
    }
}

impl Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret(<redacted>)")
    }
}

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

pub async fn store_secret(name: &str, secret: Secret) -> Result<()> {
    let entry = KeyringEntry::try_new(name)?;
    entry.set_secret(secret.0).await?;
    Ok(())
}

pub async fn get_secret(name: &str) -> Result<Option<Secret>> {
    let entry = KeyringEntry::try_new(name)?;
    if let Some(secret) = entry.find_secret().await? {
        Ok(Some(Secret(secret)))
    } else {
        Ok(None)
    }
}
