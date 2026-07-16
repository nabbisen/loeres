//! Verification gates, one module per gate (file-separation per dev guidelines).

pub mod basic;
pub mod check_rfcs;
pub mod conformance;
pub mod doc_currency;
pub mod feature_matrix;
pub mod link_audit;
pub mod no_std;
pub mod panic_audit;
pub mod public_api;
pub mod release_gate;
pub mod size_budget;
pub mod target_profiles;
pub mod unsafe_audit;
pub mod util;
pub mod zero_bleed;
