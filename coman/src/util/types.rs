use std::{collections::HashSet, fmt::Display, str::FromStr};

use color_eyre::{Report, Result};
use docker_credential::{CredentialRetrievalError, DockerCredential};
use eyre::{Context, eyre};
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::tag,
    character::complete::{alphanumeric1, digit1},
    combinator::{complete, opt, recognize},
    multi::{many_m_n, many1, separated_list0, separated_list1},
    sequence::{preceded, separated_pair, terminated},
};
use oci_client::{
    Client, Reference,
    client::{ClientConfig, ClientProtocol},
    config::ConfigFile,
    manifest::OciManifest,
    secrets::RegistryAuth,
};
use oci_spec::image::Arch;

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

impl From<Arch> for OciPlatform {
    fn from(value: Arch) -> Self {
        match value {
            Arch::ARM64 => Self::arm64,
            Arch::Amd64 => Self::amd64,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DockerImageMeta {
    pub platforms: Vec<OciPlatform>,
    pub entrypoint: Option<Vec<String>>,
    pub working_dir: Option<String>,
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
            self.registry.clone().map(|r| format!("{r}#")).unwrap_or_default(),
            self.image,
            self.tag.clone().map(|t| format!(":{}", t)).unwrap_or_default(),
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
        let (manifest, _) = client
            .pull_manifest(&reference, &auth)
            .await
            .wrap_err(format!("Couldn't get image manifest for image {reference}"))?;
        let (_, _, config) = client
            .pull_manifest_and_config(&reference, &auth)
            .await
            .wrap_err(format!("Couldn't get image config for image {reference}"))?;

        let config: ConfigFile = serde_json::from_str(&config).unwrap();
        match manifest {
            OciManifest::Image(_) => {
                //Image does not contain platform, read it from config instead (no multi-arch image)
                Ok(DockerImageMeta {
                    platforms: vec![config.clone().architecture.into()],
                    entrypoint: config.clone().config.and_then(|c| c.entrypoint),
                    working_dir: config.config.and_then(|c| c.working_dir),
                })
            }
            OciManifest::ImageIndex(oci_image_index) => {
                let mut platforms: HashSet<OciPlatform> = HashSet::new();
                platforms.extend(oci_image_index.manifests.into_iter().map(|m| {
                    m.platform
                        .map(|p| p.architecture)
                        .unwrap_or(Arch::Other("".to_owned()))
                        .into()
                }));
                Ok(DockerImageMeta {
                    platforms: platforms.into_iter().collect(),
                    entrypoint: config.clone().config.and_then(|c| c.entrypoint),
                    working_dir: config.config.and_then(|c| c.working_dir),
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
        Ok(DockerCredential::UsernamePassword(username, password)) => Ok(RegistryAuth::Basic(username, password)),
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
        // see https://ktomk.github.io/pipelines/doc/DOCKER-NAME-TAG.html#syntax
        let host = opt(terminated(
            recognize(separated_pair(
                separated_pair(alphanumeric1, tag("."), separated_list1(tag("."), alphanumeric1)),
                opt(tag(":")),
                opt(digit1),
            )),
            tag("/"),
        ));
        let image = recognize(separated_list1(
            tag("/"),
            separated_list1(
                alt((
                    tag("."),
                    recognize(many_m_n(1, 2, tag("_"))),
                    recognize(many1(tag("-"))),
                )),
                alphanumeric1,
            ),
        ));
        let docker_tag = opt(preceded(
            tag(":"),
            recognize(separated_list0(alt((tag("."), tag("-"))), alphanumeric1)),
        ));
        let digest = opt(preceded(tag("@sha256:"), alphanumeric1));
        let mut parser = complete((host, image, docker_tag, digest));
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("ubuntu",(None,"ubuntu",None,None))]
    #[case("nvidia/cuda:13.1.1-cudnn-devel-ubuntu24.04",(None,"nvidia/cuda",Some("13.1.1-cudnn-devel-ubuntu24.04"),None))]
    #[case("nvidia/cuda@sha256:deadbeef",(None,"nvidia/cuda",None,Some("deadbeef")))]
    #[case("docker.io/library/hello-world:latest@sha256:deadbeef",(Some("docker.io"),"library/hello-world",Some("latest"),Some("deadbeef")))]
    #[case("ghcr.io/swissdatasciencecenter/renku-frontend-buildpacks/run-image:0.2.1",(Some("ghcr.io"),"swissdatasciencecenter/renku-frontend-buildpacks/run-image",Some("0.2.1"),None))]
    #[case("test.ghcr.io/a/b/c/d/e:a-1.f-2", (Some("test.ghcr.io"), "a/b/c/d/e", Some("a-1.f-2"), None))]
    fn test_docker_parsing(
        #[case] docker_url: &str,
        #[case] expected: (Option<&str>, &str, Option<&str>, Option<&str>),
    ) {
        let image = DockerImageUrl::from_str(docker_url).expect("couldn't parse image");
        assert_eq!(image.registry, expected.0.map(|s| s.to_owned()));
        assert_eq!(image.image.as_str(), expected.1);
        assert_eq!(image.tag, expected.2.map(|s| s.to_owned()));
        assert_eq!(image.digest, expected.3.map(|s| s.to_owned()));
    }

    #[rstest]
    #[case((None, "ubuntu", None, None), "ubuntu")]
    #[case((Some("ghcr.io"), "test/ubuntu",Some("latest"),Some("deadbeef")), "ghcr.io#test/ubuntu:latest@sha256:deadbeef")]
    fn test_edf(#[case] values: (Option<&str>, &str, Option<&str>, Option<&str>), #[case] expected_edf: &str) {
        let image = DockerImageUrl {
            registry: values.0.map(|s| s.to_owned()),
            image: values.1.to_owned(),
            tag: values.2.map(|s| s.to_owned()),
            digest: values.3.map(|s| s.to_owned()),
        };
        assert_eq!(image.to_edf(), expected_edf);
    }
}
