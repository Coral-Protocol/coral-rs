#[allow(renamed_and_removed_lints)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/api_v1.rs"));
}

use crate::api::generated::types::AgentClaimAmount;
use std::ops::{Div, Mul};

impl AgentClaimAmount {
    pub fn is_zero(&self) -> bool {
        match self {
            AgentClaimAmount::Coral(coral) => *coral == 0.0,
            AgentClaimAmount::MicroCoral(micro) => *micro == 0,
            AgentClaimAmount::Usd(usd) => *usd == 0.0,
        }
    }
}

impl std::fmt::Display for AgentClaimAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentClaimAmount::Coral(coral) => write!(f, "{:.6} coral", coral),
            AgentClaimAmount::MicroCoral(micro) => write!(f, "{} micro-coral", micro),
            AgentClaimAmount::Usd(usd) => write!(f, "{:.6} USD", usd),
        }
    }
}

macro_rules! impl_claim_math {
    ($($t:ty),+) => {
        $(
            impl Mul<$t> for AgentClaimAmount {
                type Output = Self;

                fn mul(self, rhs: $t) -> Self::Output {
                    let rhs_f64 = rhs as f64;
                    match self {
                        AgentClaimAmount::Coral(coral) => AgentClaimAmount::Coral(coral * rhs_f64),
                        AgentClaimAmount::MicroCoral(micro) => {
                            AgentClaimAmount::MicroCoral((micro as f64 * rhs_f64) as i64)
                        },
                        AgentClaimAmount::Usd(usd) => AgentClaimAmount::Usd(usd * rhs_f64)
                    }
                }
            }

            impl Div<$t> for AgentClaimAmount {
                type Output = Self;

                fn div(self, rhs: $t) -> Self::Output {
                    let rhs_f64 = rhs as f64;
                    match self {
                        AgentClaimAmount::Coral(coral) => AgentClaimAmount::Coral(coral / rhs_f64),
                        AgentClaimAmount::MicroCoral(micro) => {
                            AgentClaimAmount::MicroCoral((micro as f64 / rhs_f64) as i64)
                        },
                        AgentClaimAmount::Usd(usd) => AgentClaimAmount::Usd(usd / rhs_f64)
                    }
                }
            }
        )+
    };
}

impl_claim_math!(f64, i64, f32, i32, u32, u64);
