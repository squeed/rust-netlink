use crate::Serializable;
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

    pub fn get_typ(&self) {}
}

// TODO: implement To<integer> methods

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
        let v = vec![1, 2, 3, 4];
        ra.add_data(&v);
        assert_eq!(ra.header.len, 0x8);

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
                1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 0, 0, 7, 8, 9, 0,
            ]
        );
    }
}
