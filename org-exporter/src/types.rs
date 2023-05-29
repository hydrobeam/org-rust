use core::fmt;

use org_parser::node_pool::{NodeID, NodePool};

pub(crate) trait Exporter<'a, T: fmt::Write> {
    fn export(input: &str) -> core::result::Result<String, fmt::Error>;
    fn export_buf<'inp, 'buf>(
        input: &'inp str,
        buf: &'buf mut T,
    ) -> core::result::Result<&'buf mut T, fmt::Error>;
    fn export_rec(&mut self, node_id: &NodeID) -> fmt::Result;

    fn buf(&mut self) -> &mut T;
    fn pool(&self) -> &NodePool<'a>;
}
