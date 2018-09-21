use failure::Error;

use super::{ChainSync, CompactBlock};

#[cfg(feature = "jsonrpc")]
mod rpc;

#[cfg(feature = "jsonrpc")]
pub use self::rpc::RpcChainSync;

pub(crate) struct MockChainSync;

impl ChainSync for MockChainSync {
    fn start_session(
        &self,
        start_height: u32,
    ) -> Result<
        (
            Box<Iterator<Item = Result<CompactBlock, Error>>>,
            Option<u32>,
        ),
        Error,
    > {
        Ok((Box::new(MockChainSession {}), None))
    }
}

struct MockChainSession;

impl Iterator for MockChainSession {
    type Item = Result<CompactBlock, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
