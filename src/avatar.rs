use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Avatar {
    Luma,
    Clack,
    #[default]
    #[serde(other)]
    Tomato,
}

impl Avatar {
    pub const ALL: [Self; 3] = [Self::Tomato, Self::Luma, Self::Clack];

    pub fn key(self) -> &'static str {
        match self {
            Self::Tomato => "tomato",
            Self::Luma => "luma",
            Self::Clack => "clack",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Tomato => "Tomato",
            Self::Luma => "Luma",
            Self::Clack => "Clack",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Tomato => "An enamel pilot with a spring in its step.",
            Self::Luma => "A lunar moth with moonlit silk wings.",
            Self::Clack => "A hermit crab tuned to pirate radio.",
        }
    }
}
