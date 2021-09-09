use lazy_static::lazy_static;
use std::env;
use web3::types::Address;

use crate::polygon::utils;

// TODO: REUSE old verification checks?
// pub(crate) const ETH_TX_SUCCESS: u64 = 1;
// pub(crate) const TRANSFER_LOGS_LENGTH: usize = 1;
// pub(crate) const TX_LOG_DATA_LENGTH: usize = 32;
// pub(crate) const TX_LOG_TOPICS_LENGTH: usize = 3;
// pub(crate) const TRANSFER_CANONICAL_SIGNATURE: &str =
//     "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

#[derive(Clone, Copy, Debug)]
pub struct EnvConfiguration {
    pub glm_contract_address: Address,
    pub glm_faucet_address: Option<Address>,
    pub required_confirmations: u64,
}

lazy_static! {
    pub static ref GOERLI_CONFIG: EnvConfiguration = EnvConfiguration {
        glm_contract_address: utils::str_to_addr(
            &env::var("GOERLI_TGLM_CONTRACT_ADDRESS")
                .unwrap_or("0x2036807B0B3aaf5b1858EE822D0e111fDdac7018".to_string())
        )
        .unwrap(),
        glm_faucet_address: Some(
            utils::str_to_addr(
                &env::var("GOERLI_TGLM_FAUCET_ADDRESS")
                    .unwrap_or("0xCCA41b09C1F50320bFB41BD6822BD0cdBDC7d85C".to_string())
            )
            .unwrap()
        ),
        required_confirmations: {
            match env::var("POLYGON_GOERLI_REQUIRED_CONFIRMATIONS").map(|s| s.parse()) {
                Ok(Ok(x)) => x,
                _ => 3,
            }
        }
    };
    pub static ref POLYGON_MAINNET_CONFIG: EnvConfiguration = EnvConfiguration {
        glm_contract_address: utils::str_to_addr(
            &env::var("POLYGON_GLM_CONTRACT_ADDRESS")
                .unwrap_or("0x0b220b82f3ea3b7f6d9a1d8ab58930c064a2b5bf".to_string())
        )
        .unwrap(),
        glm_faucet_address: None,
        required_confirmations: {
            match env::var("POLYGON_MAINNET_REQUIRED_CONFIRMATIONS").map(|s| s.parse()) {
                Ok(Ok(x)) => x,
                _ => 5,
            }
        }
    };
}