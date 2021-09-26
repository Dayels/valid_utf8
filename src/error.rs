#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UtfError {
    NotEnoughRoom,
    InvalidLead(u8),
    IncompleteSequence(u32),
    OverlongSequence(u32),
    InvalidCodePoint(u32),
}

impl std::fmt::Display for UtfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UtfError::NotEnoughRoom => write!(f, "Not enough room for validate UTF-8"),
            UtfError::InvalidLead(lead) => write!(f, "Invalid lead byte \"{:#x}\"", lead),
            UtfError::IncompleteSequence(code_point) => {
                write!(f, "Incomplete sequence \"{:#x}\"", code_point)
            }
            UtfError::OverlongSequence(code_point) => {
                write!(f, "Overlong sequence \"{:#x}\"", code_point)
            }
            UtfError::InvalidCodePoint(code_point) => {
                write!(f, "Invalid code point \"{:#x}\"", code_point)
            }
        }
    }
}

impl std::error::Error for UtfError {}
