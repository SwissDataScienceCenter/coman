use color_eyre::{Report, Result};
use docker_credential::{CredentialRetrievalError, DockerCredential};
use eyre::{Context, eyre};
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::tag,
    character::complete::{alphanumeric1, digit1},
    combinator::{complete, opt, recognize},
    multi::separated_list1,
    sequence::{preceded, terminated},
};
use oci_distribution::{
    Client, Reference,
    client::{ClientConfig, ClientProtocol},
    manifest::OciManifest,
    secrets::RegistryAuth,
};
use std::{collections::HashSet, fmt::Display, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, strum::Display)]
pub enum OciPlatform {
    #[allow(non_camel_case_types)]
    arm64,
    #[allow(non_camel_case_types)]
    amd64,
    Other,
}

impl From<String> for OciPlatform {
    fn from(value: String) -> Self {
        match value.as_str() {
            "arm64" => Self::arm64,
            "amd64" => Self::amd64,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DockerImageMeta {
    pub platforms: Vec<OciPlatform>,
}
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DockerImageUrl {
    registry: Option<String>,
    image: String,
    tag: Option<String>,
    digest: Option<String>,
}

impl DockerImageUrl {
    /// Convert to EDF form as used by CSCS
    pub fn to_edf(&self) -> String {
        format!(
            "{}{}{}{}",
            self.registry
                .clone()
                .map(|r| format!("{r}/"))
                .unwrap_or_default(),
            self.image,
            self.tag
                .clone()
                .map(|t| format!(":{}", t))
                .unwrap_or_default(),
            self.digest
                .clone()
                .map(|d| format!("@sha256:{}", d))
                .unwrap_or_default()
        )
    }

    pub async fn inspect(&self) -> Result<DockerImageMeta> {
        let client = Client::new(ClientConfig {
            protocol: ClientProtocol::Https,
            ..Default::default()
        });
        let reference = self.to_string().parse()?;
        let auth = docker_auth(&reference)?;
        let (manifest, _) = client.pull_manifest(&reference, &auth).await?;
        match manifest {
            OciManifest::Image(oci_image_manifest) => {
                // it's not clear what is returned in this case, I never hit this in my testing.
                // leaving the dbg statement so if a user ever hits this, we can ask for logs and figure it out.
                let _ = dbg!(oci_image_manifest);
                Err(eyre!(
                    "didn't get image index for image, plain manifest does not contain platform data"
                ))
            }
            OciManifest::ImageIndex(oci_image_index) => {
                let mut platforms: HashSet<OciPlatform> = HashSet::new();
                platforms.extend(oci_image_index.manifests.into_iter().map(|m| {
                    m.platform
                        .map(|p| p.architecture)
                        .unwrap_or("".to_owned())
                        .into()
                }));
                Ok(DockerImageMeta {
                    platforms: platforms.into_iter().collect(),
                })
            }
        }
    }
}

fn docker_auth(reference: &Reference) -> Result<RegistryAuth> {
    let server = reference
        .resolve_registry()
        .strip_suffix('/')
        .unwrap_or_else(|| reference.resolve_registry());
    match docker_credential::get_credential(server) {
        Ok(DockerCredential::UsernamePassword(username, password)) => {
            Ok(RegistryAuth::Basic(username, password))
        }
        Ok(DockerCredential::IdentityToken(_)) => Ok(RegistryAuth::Anonymous), // id tokens are not supported
        Err(CredentialRetrievalError::ConfigNotFound)
        | Err(CredentialRetrievalError::NoCredentialConfigured)
        | Err(CredentialRetrievalError::ConfigReadError) => Ok(RegistryAuth::Anonymous),
        Err(e) => Err(e).wrap_err("couldn't get docker credentials"),
    }
}

type DockerParseType<'a> = IResult<
    &'a str,
    (
        Option<&'a str>, // domain[:port]
        &'a str,         // [namespace/]image
        Option<&'a str>, // tag
        Option<&'a str>, // digest
    ),
>;

impl FromStr for DockerImageUrl {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = complete((
            opt(terminated(
                alt((
                    recognize((separated_list1(tag("."), alphanumeric1), tag(":"), digit1)),
                    recognize(separated_list1(tag("."), alphanumeric1)),
                )),
                tag("/"),
            )),
            alt((
                recognize((alphanumeric1, tag("/"), alphanumeric1)),
                alphanumeric1,
            )),
            opt(preceded(tag(":"), alphanumeric1)),
            opt(preceded(tag("@sha256:"), alphanumeric1)),
        ));
        let parsed: DockerParseType = parser.parse(s);
        match parsed {
            Ok(result) => Ok(DockerImageUrl {
                registry: result.1.0.map(|r| r.to_owned()),
                image: result.1.1.to_owned(),
                tag: result.1.2.map(|t| t.to_owned()),
                digest: result.1.3.map(|d| d.to_owned()),
            }),
            Err(e) => Err(eyre!("couldn't parse docker image url: {e}")),
        }
    }
}

impl TryFrom<String> for DockerImageUrl {
    type Error = Report;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        DockerImageUrl::from_str(&value)
    }
}

impl Display for DockerImageUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(registry) = self.registry.as_ref() {
            write!(f, "{}/", registry)?;
        }

        write!(f, "{}", self.image)?;

        if let Some(tag) = self.tag.as_ref() {
            write!(f, ":{}", tag)?;
        }

        if let Some(digest) = self.digest.as_ref() {
            write!(f, "@sha256:{}", digest)?;
        }
        Ok(())
    }
}
