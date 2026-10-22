/// Family ID for the R8 family.
pub const FAMILY_R8: u8 = 0x01;

// Model ID for the R8 model 1.
pub const MODEL_R8_1: u8 = 0x01;

/// Query number for the CPUID 'capabilities' query.
pub const QUERY_CAPABILITIES: u8 = 0x04;

/// Query number for the CPUID 'extended ISA pages' query.
pub const QUERY_EXTENDED: u8 = 0x03;

/// Query number for the CPUID 'highest function' query.
pub const QUERY_HIGHEST: u8 = 0x00;

/// Query number for the CPUID 'family / model / revision' query.
pub const QUERY_MODEL: u8 = 0x02;

/// Query number for the CPUID 'vendor ID' query.
pub const QUERY_VENDOR: u8 = 0x01;

/// Trap code for the 'illegal instruction' trap.
pub const TRAP_ILLEGAL: u8 = 0x00;

/// Trap code for the 'overflow' trap.
pub const TRAP_OVERFLOW: u8 = 0x01;

/// Trap code for the 'panic' trap.
pub const TRAP_PANIC: u8 = 0x02;

/// The hard-wired reset vector address.
pub const VEC_RESET: u16 = 0xFFFE;

/// Vendor ID for SST, Inc.
pub const VENDOR_SSTI: &[u8; 8] = b"SSTi    ";
