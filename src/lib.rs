use pinocchio_pubkey::declare_id;

#[cfg(not(feature = "no-entrypoint"))]
#[cfg(feature = "std")]
extern crate std;

mod entrypoint;
mod error;
mod instructions;
mod states;
mod types;

declare_id!("E6UcK3dSFc2yaFtEb35pc1WsBVcrPhEbnB87YoNDXhqy");
