use core::fmt;

use org_parser::node_pool::{NodeID, NodePool};

pub(crate) trait Exporter<'a> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error>;
    fn export_buf<'inp, 'buf, T: fmt::Write>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error>;
    fn export_rec(&mut self, node_id: &NodeID, buf: &mut dyn fmt::Write) -> fmt::Result;

    fn pool(&self) -> &NodePool<'a>;

    fn write(&mut self,  buf: &mut dyn fmt::Write, s: &str) -> fmt::Result;
}
