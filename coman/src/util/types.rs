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
    sequence::{preceded, terminated},
};
use oci_distribution::{
    Client, Reference,
    client::{ClientConfig, ClientProtocol},
    manifest::OciManifest,
    secrets::RegistryAuth,
};

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
                platforms.extend(
                    oci_image_index
                        .manifests
                        .into_iter()
                        .map(|m| m.platform.map(|p| p.architecture).unwrap_or("".to_owned()).into()),
                );
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
            alt((
                recognize((separated_list1(tag("."), alphanumeric1), tag(":"), digit1)),
                recognize(separated_list1(tag("."), alphanumeric1)),
            )),
            tag("/"),
        ));
        let image = recognize(separated_list0(
            tag("/"),
            separated_list0(
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
