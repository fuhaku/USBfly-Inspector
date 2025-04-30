// USB decoder module
// This is a temporary placeholder while we migrate to new implementation

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Speed {
    Auto = 0,
    High = 1,
    Full = 2,
    Low = 3,
}

impl Speed {
    pub fn mask(&self) -> u8 {
        1 << (*self as u8)
    }
}