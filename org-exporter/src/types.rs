use core::fmt;

use org_parser::node_pool::{NodeID, NodePool};

pub(crate) trait Exporter<'a, 'buf> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error>;
    fn export_buf<'inp, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error>;
    fn export_rec(&mut self, node_id: &NodeID) -> fmt::Result;

    fn buf(&mut self) -> &mut dyn fmt::Write;
    fn pool(&self) -> &NodePool<'a>;
}
