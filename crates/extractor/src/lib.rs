//! Taikoscope Extractor
use chainio::{self, ITaikoInbox::BatchProposed, taiko::preconf_whitelist::TaikoPreconfWhitelist};

use std::pin::Pin;

use alloy::{
    primitives::{Address, BlockHash},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::eth::Log,
};
use chainio::TaikoInbox;
use derive_more::Debug;
use eyre::Result;
use tokio_stream::{Stream, StreamExt};
use tracing::info;
use url::Url;
/// Extractor client
#[derive(Debug)]
pub struct Extractor {
    #[debug(skip)]
    l1_provider: Box<dyn Provider + Send + Sync>,
    #[debug(skip)]
    l2_provider: Box<dyn Provider + Send + Sync>,
    preconf_whitelist: TaikoPreconfWhitelist,
    taiko_inbox: TaikoInbox,
}

/// L1 Header
#[derive(Debug)]
pub struct L1Header {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block slot
    pub slot: u64,
    /// Extracted block timestamp
    pub timestamp: u64,
}

/// L2 Header
#[derive(Debug)]
pub struct L2Header {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: BlockHash,
    /// Block timestamp
    pub timestamp: u64,
    /// Gas used
    pub gas_used: u64,
    /// Beneficiary
    pub beneficiary: Address,
}

impl Extractor {
    /// Create a new extractor
    pub async fn new(
        l1_rpc_url: Url,
        l2_rpc_url: Url,
        inbox_address: Address,
        preconf_whitelist_address: Address,
    ) -> Result<Self> {
        let l1_el = WsConnect::new(l1_rpc_url.clone());
        let l2_el = WsConnect::new(l2_rpc_url);
        let l1_provider = ProviderBuilder::new().connect_ws(l1_el).await?;
        let l2_provider = ProviderBuilder::new().connect_ws(l2_el).await?;

        let taiko_inbox = TaikoInbox::new_readonly(inbox_address, l1_provider.clone());
        let preconf_whitelist =
            TaikoPreconfWhitelist::from_address(l1_rpc_url, preconf_whitelist_address);

        Ok(Self {
            l1_provider: Box::new(l1_provider),
            l2_provider: Box::new(l2_provider),
            preconf_whitelist,
            taiko_inbox,
        })
    }

    /// Get a stream of L1 headers
    pub async fn get_l1_header_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = L1Header> + Send>>> {
        // Subscribe to new blocks
        let sub = self.l1_provider.subscribe_blocks().await?;
        let stream = sub.into_stream();
        info!("Subscribed to L1 block headers");

        // Convert stream to header stream
        let header_stream = stream.map(|header| L1Header {
            number: header.number,
            hash: header.hash,
            slot: header.number, // TODO: Get slot instead
            timestamp: header.timestamp,
        });

        Ok(Box::pin(header_stream))
    }

    /// Get a stream of L2 headers
    pub async fn get_l2_header_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = L2Header> + Send>>> {
        // Subscribe to new blocks
        let sub = self.l2_provider.subscribe_blocks().await?;
        let stream = sub.into_stream();
        info!("Subscribed to L2 block headers");

        // Convert stream to header stream
        let header_stream = stream.map(|header| L2Header {
            number: header.number,
            hash: header.hash,
            timestamp: header.timestamp,
            gas_used: header.gas_used,
            beneficiary: header.beneficiary,
        });

        Ok(Box::pin(header_stream))
    }

    /// Subscribes to the `TaikoInbox`  `BatchProposed` event and returns a stream of decoded
    /// events.
    pub async fn get_batch_proposed_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = BatchProposed> + Send>>> {
        let filter = self.taiko_inbox.batch_proposed_filter();
        let logs = self.l1_provider.subscribe_logs(&filter).await?.into_stream();

        // Convert stream to batch proposed stream
        let batch_proposed_stream =
            logs.filter_map(|log: Log| match log.log_decode::<BatchProposed>() {
                Ok(decoded) => {
                    // Extract the BatchProposed event from the Log<BatchProposed>
                    Some(decoded.data().clone())
                }
                Err(err) => {
                    tracing::warn!("Failed to decode log: {}", err);
                    None
                }
            });

        Ok(Box::pin(batch_proposed_stream))
    }

    /// Get the current epoch operator
    pub async fn get_operator_for_current_epoch(&self) -> Result<Address> {
        let operator = self.preconf_whitelist.get_operator_for_current_epoch().await?;
        Ok(operator)
    }

    /// Get the next epoch operator
    pub async fn get_operator_for_next_epoch(&self) -> Result<Address> {
        let operator = self.preconf_whitelist.get_operator_for_next_epoch().await?;
        Ok(operator)
    }

    /// Get the operator candidates for the current epoch
    pub async fn get_operator_candidates_for_current_epoch(&self) -> Result<Vec<Address>> {
        let candidates = self.preconf_whitelist.get_operator_candidates_for_current_epoch().await?;
        Ok(candidates)
    }
}
