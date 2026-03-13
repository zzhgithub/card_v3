use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardIdParseError {
    InvalidFormat,
    InvalidPackNumber,
    InvalidCardType,
    InvalidCardNumber,
}

impl fmt::Display for CardIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => {
                write!(f, "Invalid CardId format, expected S[pack]-[type]-[num]")
            }
            Self::InvalidPackNumber => write!(f, "Invalid pack number, expected 3 digits"),
            Self::InvalidCardType => write!(f, "Invalid card type, expected C/S/I/L"),
            Self::InvalidCardNumber => write!(f, "Invalid card number, expected 3 digits"),
        }
    }
}

impl std::error::Error for CardIdParseError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardId(pub String);

impl CardId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn pack(&self) -> Option<&str> {
        self.0.split('-').next()
    }

    pub fn card_type(&self) -> Option<&str> {
        self.0.split('-').nth(1)
    }

    pub fn card_number(&self) -> Option<&str> {
        self.0.split('-').nth(2)
    }
}

impl fmt::Display for CardId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for CardId {
    type Err = CardIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(CardIdParseError::InvalidFormat);
        }

        let pack = parts[0];
        if !pack.starts_with('S') || pack.len() != 4 {
            return Err(CardIdParseError::InvalidPackNumber);
        }
        if !pack[1..].chars().all(|c| c.is_ascii_digit()) {
            return Err(CardIdParseError::InvalidPackNumber);
        }

        let card_type = parts[1];
        if card_type.len() != 1 || !matches!(card_type, "C" | "S" | "I" | "L") {
            return Err(CardIdParseError::InvalidCardType);
        }

        let card_number = parts[2];
        if card_number.len() != 3 || !card_number.chars().all(|c| c.is_ascii_digit()) {
            return Err(CardIdParseError::InvalidCardNumber);
        }

        Ok(CardId(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_card_id() {
        let id: CardId = "S001-C-001".parse().unwrap();
        assert_eq!(id.pack(), Some("S001"));
        assert_eq!(id.card_type(), Some("C"));
        assert_eq!(id.card_number(), Some("001"));
    }

    #[test]
    fn test_invalid_format() {
        assert!("001-C-001".parse::<CardId>().is_err());
        assert!("S001-C".parse::<CardId>().is_err());
    }

    #[test]
    fn test_invalid_card_type() {
        assert!("S001-X-001".parse::<CardId>().is_err());
    }

    #[test]
    fn test_invalid_card_number() {
        assert!("S001-C-00".parse::<CardId>().is_err());
        assert!("S001-C-0001".parse::<CardId>().is_err());
    }

    #[test]
    fn test_display() {
        let id = CardId::new("S001-C-001");
        assert_eq!(id.to_string(), "S001-C-001");
    }
}
