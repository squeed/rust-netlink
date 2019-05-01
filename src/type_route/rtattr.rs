use crate::Serializable;
use std::ffi::{CStr, CString};
use std::io::{Error, ErrorKind, Result};
use std::ptr;

#[repr(C)]
#[derive(Debug)]
/// RtAttr is the length-type-value struct that holds data.
pub struct RtAttr {
    header: RtAttrHeader,
    pub data: Vec<u8>,
}

#[repr(C)]
#[derive(Debug)]
struct RtAttrHeader {
    pub len: u16,
    pub typ: u16,
}

impl RtAttrHeader {
    pub fn size() -> usize {
        0x4
    }

    pub fn data_size(&self) -> usize {
        return self.len as usize - 4;
    }

    pub fn from_bytes(v: &[u8]) -> Result<RtAttrHeader> {
        if v.len() < RtAttrHeader::size() {
            return Err(Error::new(ErrorKind::UnexpectedEof, "message too short"));
        }

        let mem = v.to_owned();
        let h: RtAttrHeader = unsafe { ptr::read(mem.as_ptr() as *mut RtAttrHeader) };

        Ok(h)
    }
}

impl RtAttr {
    // question: is there a good way to make data generic or
    // more user-friendly? Could you write something like
    // new(IFLA_MTU, 1500)
    // and have it automatically convert to bytes
    pub fn new(typ: u16, data: Vec<u8>) -> RtAttr {
        RtAttr {
            header: RtAttrHeader {
                len: 0x4 + data.len() as u16,
                typ: typ,
            },
            data: data,
        }
    }

    pub fn add_data<S: Serializable>(&mut self, data: &S) {
        let mut d = data.to_bytes();
        let l = d.len();
        let aligned_len = crate::util::align(l);
        let padding = aligned_len - l;

        self.data.append(&mut d);
        for _ in 0..padding {
            self.data.push(0);
        }
        self.header.len += aligned_len as u16;
    }

    pub fn get_typ(&self) -> u16 {
        self.header.typ
    }

    pub fn as_u32(&self) -> Result<u32> {
        if self.header.data_size() < 4 {
            return Err(Error::new(ErrorKind::InvalidData, ""));
        }

        let mut d: [u8; 4] = [0; 4];
        d.copy_from_slice(&self.data[0..4]);
        Ok(u32::from_ne_bytes(d))
    }

    pub fn as_u16(&self) -> Result<u16> {
        if self.header.data_size() < 2 {
            return Err(Error::new(ErrorKind::InvalidData, ""));
        }

        let mut d: [u8; 2] = [0; 2];
        d.copy_from_slice(&self.data[0..2]);
        Ok(u16::from_ne_bytes(d))
    }

    pub fn as_bool(&self) -> Result<bool> {
        if self.header.data_size() == 0 {
            return Err(Error::new(ErrorKind::InvalidData, ""));
        }
        Ok(self.data[0] == 1)
    }

    pub fn to_cstring(&self) -> Result<CString> {
        let cstr = match CStr::from_bytes_with_nul(&self.data) {
            Ok(cstr) => cstr,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "invalid interface name")),
        };
        Ok(CString::from(cstr))
    }

    pub fn one_from_bytes(v: &[u8], idx: usize) -> Result<RtAttr> {
        if v.len() < (idx + RtAttrHeader::size()) {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "message too short for rtattr header",
            ));
        }

        let header = RtAttrHeader::from_bytes(&v[idx..idx + RtAttrHeader::size()])?;
        let header_len = header.len as usize;
        if v.len() < (idx + header_len) {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "buffer too short for message",
            ));
        }

        // the leftover data is [idx + header .. idx +  len]
        let attr = RtAttr {
            header: header,
            data: v[idx + RtAttrHeader::size()..idx + header_len].to_owned(),
        };
        return Ok(attr);
    }

    pub fn from_bytes(v: &[u8]) -> Result<Vec<RtAttr>> {
        let mut idx = 0;
        let mut res = Vec::new();
        let len = v.len();

        while idx < len {
            let msg = RtAttr::one_from_bytes(v, idx)?;
            idx += msg.header.len as usize;
            idx = crate::util::align(idx);
            res.push(msg);
        }

        Ok(res)
    }
}

impl crate::Serializable for RtAttr {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(self.header.len as usize);

        // poop the header to the head of the vector
        unsafe {
            ptr::copy_nonoverlapping(&self.header, out.as_mut_ptr() as *mut RtAttrHeader, 1);
            out.set_len(RtAttrHeader::size());
        }

        out.extend(self.data.iter());
        return out;
    }
}

#[cfg(test)]
mod tests {
    use super::RtAttr;
    use crate::Serializable;
    #[test]
    fn test_rtattr() {
        let mut ra = RtAttr::new(1, vec![]);

        assert_eq!(ra.header.typ, 1);
        assert_eq!(ra.header.len, 0x4);

        // Add some data
        let v = vec![0x12, 0x34, 0x56, 0x78];
        ra.add_data(&v);
        assert_eq!(ra.header.len, 0x8);
        assert_eq!(ra.header.data_size(), 4);
        let d = ra.as_u32();
        assert_eq!(d.is_ok(), true);
        assert_eq!(
            d.unwrap(),
            if cfg!(target_endian = "big") {
                0x12345678
            } else {
                0x78563412
            }
        );

        // Add some unaligned data
        let v = vec![1, 2, 3, 4, 5, 6];
        ra.add_data(&v);
        assert_eq!(ra.header.len, 0x10);

        let v = vec![7, 8, 9];
        ra.add_data(&v);

        assert_eq!(
            ra.to_bytes(),
            vec![
                0x14, 0, //len
                1, 0, //typ
                0x12, 0x34, 0x56, 0x78, 1, 2, 3, 4, 5, 6, 0, 0, 7, 8, 9, 0,
            ]
        );
    }
}
