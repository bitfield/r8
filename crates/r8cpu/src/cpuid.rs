#[derive(Clone, Copy, Debug)]
pub struct Capabilities {
    pub emulation: bool,
}

impl From<Capabilities> for u8 {
    fn from(cap: Capabilities) -> Self {
        u8::from(cap.emulation)
    }
}

#[derive(Debug)]
pub struct Cpuid {
    pub capabilities: Capabilities,
    pub extended: u8,
    pub family: u8,
    pub highest: u8,
    pub model: u8,
    pub revision: u8,
    pub vendor: &'static [u8; 8],
}
