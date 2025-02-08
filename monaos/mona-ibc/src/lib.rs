use ibc::{
    core::{
        ics02_client::client_state::ClientState,
        ics24_host::identifier::{ChainId, ClientId},
    },
    Height,
};

/// IBC Client configuration
#[derive(Debug, Clone)]
pub struct IbcClientConfig {
    pub chain_id: ChainId,
    pub client_id: ClientId,
    pub height: Height,
}

impl IbcClientConfig {
    pub fn new(chain_id: &str, client_id: &str, revision_number: u64, revision_height: u64) -> Result<Self, String> {
        Ok(Self {
            chain_id: ChainId::new(chain_id.to_string()).map_err(|e| e.to_string())?,
            client_id: ClientId::new(client_id.to_string()).map_err(|e| e.to_string())?,
            height: Height::new(revision_number, revision_height).map_err(|e| e.to_string())?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibc_client_config() {
        let config = IbcClientConfig::new(
            "test-chain", 
            "07-tendermint-0", 
            0, 
            1
        ).unwrap();

        assert_eq!(config.chain_id.to_string(), "test-chain");
        assert_eq!(config.client_id.to_string(), "07-tendermint-0");
        assert_eq!(config.height.revision_number(), 0);
        assert_eq!(config.height.revision_height(), 1);
    }
}
