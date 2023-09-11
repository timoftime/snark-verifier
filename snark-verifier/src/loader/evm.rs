//! `Loader` implementation for generating yul code as EVM verifier.

mod code;
pub(crate) mod loader;
pub(crate) mod util;

#[cfg(test)]
mod test;

pub use loader::{EcPoint, EvmLoader, Scalar};
pub use util::{
    compile_solidity, deploy_and_call, encode_calldata, estimate_gas, fe_to_u256, modulus,
    u256_to_fe, Address, B256, U256, U512,
};

// #[cfg(test)]
// pub use test::execute;
