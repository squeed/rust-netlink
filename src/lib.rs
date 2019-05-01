#![allow(dead_code)]
pub mod hl;
pub mod proto;
pub mod type_route;
pub mod uapi;

pub trait Serializable {
    fn to_bytes(&self) -> Vec<u8>;
}

impl Serializable for std::vec::Vec<u8> {
    fn to_bytes(&self) -> Vec<u8> {
        return self.to_owned();
    }
}

#[cfg(test)]
mod tests {

    use crate::proto::NetlinkHeader;
    #[test]
    fn it_works() {
        let x = NetlinkHeader {
            len: 0,
            typ: 0,
            flags: 0,
            seq: 0,
            pid: 0,
        };
        assert_eq!(x.len, 0);
    }
}

pub mod util {
    pub fn align(len: usize) -> usize {
        const RTA_ALIGNTO: usize = 4;

        ((len) + RTA_ALIGNTO - 1) & !(RTA_ALIGNTO - 1)
    }

}
