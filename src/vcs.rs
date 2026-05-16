use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Vcs {
    Github,
    Gitlab,
}

impl fmt::Display for Vcs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Github => write!(formatter, "github"),
            Self::Gitlab => write!(formatter, "gitlab"),
        }
    }
}

impl FromStr for Vcs {
    type Err = anyhow::Error;

    fn from_str(vcs_text: &str) -> Result<Self, Self::Err> {
        match vcs_text.to_lowercase().as_str() {
            "github" => Ok(Self::Github),
            "gitlab" => Ok(Self::Gitlab),
            unknown_vcs => bail!("invalid vcs: {unknown_vcs} (expected 'github' or 'gitlab')"),
        }
    }
}
