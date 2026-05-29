use actrpc_transport::TransportTarget;

#[derive(Debug, Clone)]
pub struct Destination {
    pub target: TransportTarget,
}
