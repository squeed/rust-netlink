use crate::Serializable;
use std::io::{Error, ErrorKind, Result};
use std::mem;
use std::ptr;

/// The preamble packet sent with every netlink transaction
#[repr(C)]
#[derive(Debug, Eq, Clone)]
pub struct NetlinkHeader {
    // TODO just use libc::nlmsghdr
    pub len: u32,
    pub typ: u16,
    pub flags: u16,
    pub seq: u32,
    pub pid: u32,
}

impl NetlinkHeader {
    pub fn from_bytes(v: &[u8]) -> Result<NetlinkHeader> {
        if v.len() < NetlinkHeader::size() {
            return Err(Error::new(ErrorKind::UnexpectedEof, "message too short"));
        }

        // Duplicate bytes, transmute to netlink header
        let mem = v.to_owned();
        let h: NetlinkHeader = unsafe { ptr::read(mem.as_ptr() as *mut NetlinkHeader) };

        Ok(h)
    }

    pub fn size() -> usize {
        mem::size_of::<NetlinkHeader>()
    }
}

impl crate::Serializable for NetlinkHeader {
    fn to_bytes(&self) -> Vec<u8> {
        let s = NetlinkHeader::size();
        if (self.len as usize) < s {
            panic!("invalid message length");
        }

        // We will append the rest of the message to this vector, so we might
        // as well allocate the whole thing now
        let mut out: Vec<u8> = Vec::with_capacity(self.len as usize);
        unsafe {
            ptr::copy_nonoverlapping(self, out.as_mut_ptr() as *mut NetlinkHeader, 1);
            out.set_len(s);
        }
        return out;
    }
}

impl std::cmp::PartialEq for NetlinkHeader {
    fn eq(&self, other: &NetlinkHeader) -> bool {
        self.len == other.len
            && self.typ == other.typ
            && self.flags == other.flags
            && self.seq == other.seq
            && self.pid == other.pid
    }
}

/// NetlinkMessage is a single netlink message sent or received over the socket.
#[derive(Debug, Clone)]
pub struct NetlinkMessage {
    pub header: NetlinkHeader,
    pub data: Vec<u8>, // the remaining data
}

impl NetlinkMessage {
    pub fn new(typ: u16, flags: u16) -> NetlinkMessage {
        NetlinkMessage {
            header: NetlinkHeader {
                len: NetlinkHeader::size() as u32,
                typ: typ,
                flags: flags,
                seq: 0, // set when sending
                pid: 0, // set by kernel
            },
            data: vec![],
        }
    }

    pub fn from_bytes(v: &[u8]) -> Result<Vec<NetlinkMessage>> {
        let mut idx = 0;
        let mut res = Vec::new();
        let len = v.len();

        while idx < len {
            let msg = NetlinkMessage::one_from_bytes(v, idx)?;
            idx += msg.header.len as usize;
            idx = crate::util::align(idx);
            res.push(msg);
        }

        Ok(res)
    }

    pub fn one_from_bytes(v: &[u8], idx: usize) -> Result<NetlinkMessage> {
        if v.len() < (idx + NetlinkHeader::size()) {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "message too short for header",
            ));
        }

        // read the header pointing at idx
        let header = NetlinkHeader::from_bytes(&v[idx..idx + NetlinkHeader::size()])?;
        let header_len = header.len as usize;
        if v.len() < (idx + header_len) {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "buffer too short for message",
            ));
        }

        // the leftover data is [idx + header .. idx +  len]
        let pkt = NetlinkMessage {
            header: header,
            data: v[idx + NetlinkHeader::size()..idx + header_len].to_owned(),
        };
        return Ok(pkt);
    }

    /// Adds some raw data to the netlink message, and updates length.
    /// This adds any necessary padding after the appended data to ensure
    /// it matches the netlink alignment rules.
    pub fn add_data(&mut self, mut d: Vec<u8>) {
        let l = d.len();
        let aligned_len = crate::util::align(l);
        let padding = aligned_len - l;

        // Netlink messages are always aligned; pad with zeroes.
        self.data.append(&mut d);
        for _ in 0..padding {
            self.data.push(0);
        }
        self.header.len += aligned_len as u32;
    }
}

impl Serializable for NetlinkMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.header.to_bytes();
        out.extend(self.data.iter());
        return out;
    }
}

#[cfg(test)]
mod tests {
    use super::NetlinkHeader;
    use super::NetlinkMessage;
    use crate::Serializable;

    #[test]
    fn test_from_one() {
        // TODO: big-endian machines
        let mut b = vec![
            0x10, 0, 0, 0, //len
            2, 0, // type
            3, 0, // flags
            4, 0, 0, 0, // sequence
            5, 0, 0, 0, // pid
        ];

        let p = NetlinkMessage::one_from_bytes(&b, 0);
        assert!(p.is_ok());
        let p = p.unwrap();
        assert_eq!(
            p.header,
            NetlinkHeader {
                len: 0x10,
                typ: 2,
                flags: 3,
                seq: 4,
                pid: 5,
            }
        );
        assert_eq!(p.data.len(), 0);

        // Add a byte of data to b
        b.push(6);
        b[0] = 0x11; // adjust length
        let p = NetlinkMessage::one_from_bytes(&b, 0).unwrap();
        assert_eq!(p.data, vec![6]);
        assert_eq!(p.data, vec![6]);
    }

    #[test]
    fn test_from_bytes() {
        let b = vec![
            0x14, 0, 0, 0, //len
            2, 0, // type
            3, 0, // flags
            4, 0, 0, 0, // sequence
            5, 0, 0, 0, // pid
            6, 0, 0, 0, // extra data
            0x10, 0, 0, 0, //len
            22, 0, // type
            33, 0, // flags
            44, 0, 0, 0, // sequence
            55, 0, 0, 0, // pid
        ];
        let msgs = NetlinkMessage::from_bytes(&b);
        assert!(msgs.is_ok());
        let msgs = msgs.unwrap();

        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].header.len, 0x14);
        assert_eq!(msgs[0].header.pid, 5);
        assert_eq!(msgs[0].data, vec![6, 0, 0, 0]);
        assert_eq!(msgs[1].header.pid, 55);
    }

    #[test]
    fn test_header_serialize() {
        let h = NetlinkHeader {
            len: 0x20,
            typ: 2,
            flags: 99,
            seq: 4,
            pid: 5,
        };

        let b = h.to_bytes();
        assert_eq!(b.len(), NetlinkHeader::size());
        assert_eq!(b.capacity(), 0x20);
        assert_eq!(
            b,
            vec![
                0x20, 0, 0, 0, //len
                2, 0, // type
                99, 0, // flags
                4, 0, 0, 0, // sequence
                5, 0, 0, 0, // pid
            ]
        );
    }

    #[test]
    fn test_message_serialize() {
        let mut m = NetlinkMessage::new(42, 33);
        m.add_data(vec![1, 2, 3, 4]);

        let b = m.to_bytes();
        assert_eq!(
            b,
            vec![
                0x14, 0, 0, 0, //len
                42, 0, //type
                33, 0, //flags
                0, 0, 0, 0, // sequence
                0, 0, 0, 0, // pid
                1, 2, 3, 4, // extra data
            ]
        );
    }

    #[test]
    fn test_add_data() {
        let mut m = NetlinkMessage::new(99, 88);
        m.add_data(vec![1, 2, 3, 4]);

        assert_eq!(m.header.len, 0x14);
        assert_eq!(m.data, vec![1, 2, 3, 4]);

        // now test unaligned data
        m.add_data(vec![42]);
        assert_eq!(m.header.len, 0x18);
        assert_eq!(m.data, vec![1, 2, 3, 4, 42, 0, 0, 0]);
    }
}
