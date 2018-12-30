use std::io::{Error, ErrorKind, Result};
use std::ptr;

#[repr(C)]
#[derive(Debug, Eq, Clone, Default)]
pub struct IfInfoMsg {
    pub family: u8,
    _pad: u8,
    pub typ: u16,
    pub index: i32,
    pub flags: u32,
    pub change: u32,
}

impl IfInfoMsg {
    pub fn from_bytes(v: &[u8]) -> Result<IfInfoMsg> {
        if v.len() < IfInfoMsg::size() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "buffer too short for message",
            ));
        }

        let mem = v.to_owned();
        let m: IfInfoMsg = unsafe { std::ptr::read(mem.as_ptr() as *mut IfInfoMsg) };

        Ok(m)
    }

    pub fn size() -> usize {
        0x10
    }
}

impl std::cmp::PartialEq for IfInfoMsg {
    fn eq(&self, other: &IfInfoMsg) -> bool {
        self.family == other.family
            && self.typ == other.typ
            && self.index == other.index
            && self.flags == other.flags
            && self.change == other.change
    }
}

impl crate::Serializable for IfInfoMsg {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::with_capacity(IfInfoMsg::size());
        unsafe {
            ptr::copy_nonoverlapping(self, out.as_mut_ptr() as *mut IfInfoMsg, 1);
            out.set_len(IfInfoMsg::size());
        };
        return out;
    }
}

#[cfg(test)]
mod tests {
    use super::IfInfoMsg;
    use crate::Serializable;
}
