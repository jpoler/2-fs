use std::ops::Add;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Hash)]
pub struct Cluster(u32);

impl From<u32> for Cluster {
    fn from(raw_num: u32) -> Cluster {
        Cluster(raw_num & !(0xF << 28))
    }
}

impl Cluster {
    pub fn get(&self) -> u32 {
        self.0
    }
}

impl Add for Cluster {
    type Output = Cluster;

    fn add(self, other: Cluster) -> Cluster {
        Cluster::from(self.0 + other.0)
    }
}
