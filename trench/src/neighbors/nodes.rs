

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighborNode {
    pub id: String,
    pub address: String,
    pub port: u16,
    pub public_key: String,
    pub last_seen: Option<u64>,
    pub status: String,
    pub version: String,
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighborNodeList {
    pub nodes: Vec<NeighborNode>,
}

