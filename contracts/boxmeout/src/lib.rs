// contract/src/lib.rs - BoxMeOut Stella - Main Contract Entry Point
// Soroban WASM smart contracts for prediction market platform on Stellar

#![no_std]

// Module declarations for modular contract architecture
// NOTE: Only one contract can be compiled at a time for WASM
// To build different contracts, comment/uncomment the appropriate module

// FACTORY CONTRACT (currently active)
pub mod factory;
pub use factory::*;

// Uncomment below to build other contracts:
pub mod market;
pub use market::*;

pub mod treasury;
pub use treasury::*;

pub mod oracle;
pub use oracle::*;

pub mod amm;
pub use amm::*;

pub mod helpers;
pub use helpers::*;
