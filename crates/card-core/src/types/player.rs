use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerId {
    Player1,
    Player2,
}

impl PlayerId {
    pub fn opponent(self) -> Self {
        match self {
            PlayerId::Player1 => PlayerId::Player2,
            PlayerId::Player2 => PlayerId::Player1,
        }
    }
}

/// Custom deserializer that handles both string format ("Self_") and internally-tagged format ({"type": "Self_"})
fn deserialize_player_ref<'de, D>(deserializer: D) -> Result<PlayerRef, D::Error>
where
    D: Deserializer<'de>,
{
    struct PlayerRefVisitor;

    impl<'de> serde::de::Visitor<'de> for PlayerRefVisitor {
        type Value = PlayerRef;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string 'Self_' or 'Opponent', or a map with 'type' field")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "Self_" => Ok(PlayerRef::Self_),
                "Opponent" => Ok(PlayerRef::Opponent),
                _ => Err(serde::de::Error::unknown_variant(
                    value,
                    &["Self_", "Opponent"],
                )),
            }
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            // Deserialize as internally-tagged struct
            #[derive(Deserialize)]
            #[serde(tag = "type")]
            enum TaggedPlayerRef {
                Self_,
                Opponent,
            }

            let tagged: TaggedPlayerRef = TaggedPlayerRef::deserialize(
                serde::de::value::MapAccessDeserializer::new(map),
            )?;

            match tagged {
                TaggedPlayerRef::Self_ => Ok(PlayerRef::Self_),
                TaggedPlayerRef::Opponent => Ok(PlayerRef::Opponent),
            }
        }
    }

    deserializer.deserialize_any(PlayerRefVisitor)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum PlayerRef {
    Self_,
    Opponent,
}

impl<'de> Deserialize<'de> for PlayerRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_player_ref(deserializer)
    }
}

impl PlayerRef {
    pub fn resolve(self, current_player: PlayerId) -> PlayerId {
        match self {
            PlayerRef::Self_ => current_player,
            PlayerRef::Opponent => current_player.opponent(),
        }
    }
}
