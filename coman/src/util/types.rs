use color_eyre::{Report, Result};
use eyre::eyre;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::tag,
    character::complete::{alphanumeric1, digit1},
    combinator::{complete, opt, recognize},
    multi::separated_list1,
    sequence::{preceded, terminated},
};
use std::str::FromStr;

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
