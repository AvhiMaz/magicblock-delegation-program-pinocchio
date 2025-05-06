pub mod delegate;

pub use delegate::*;

use pinocchio::program_error::ProgramError;

#[repr(u8)]
pub enum DelegateProgram {
    Delegate,
}

impl TryFrom<&u8> for DelegateProgram {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(DelegateProgram::Delegate),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
