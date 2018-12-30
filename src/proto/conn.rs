use crate::Serializable;
use byteorder::{NativeEndian, ReadBytesExt};
use libc;
use std::io::{Cursor, Error, ErrorKind, Result};
use std::mem;
use std::os::unix::io::RawFd;

const RECEIVE_BUFFER_SIZE: usize = 65536;

#[derive(Debug)]
pub struct NetlinkSocket {
    proto: i32,
    next_seq: u32,
    fd: RawFd,
}

impl NetlinkSocket {
    pub fn new(proto: i32) -> Result<NetlinkSocket> {
        let mut s = NetlinkSocket {
            next_seq: 0,
            proto: proto,
            fd: 0,
        };

        return s.bind().and(Ok(s));
    }

    fn sockaddr(&self) -> libc::sockaddr_nl {
        let mut saddr: libc::sockaddr_nl = unsafe { mem::zeroed() };
        saddr.nl_family = libc::AF_NETLINK as libc::sa_family_t;

        saddr
    }

    /// The so-called port id, assigned by the kernel when the socket is opened.
    /// Has nothing to do with the process id.
    fn pid(&self) -> Result<u32> {
        if self.fd <= 0 {
            return Err(Error::new(ErrorKind::NotConnected, "not connected"));
        }

        let mut saddr = self.sockaddr();
        let slen = mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t;
        let res = unsafe {
            libc::getsockname(self.fd, mem::transmute(&mut saddr), mem::transmute(&slen))
        };

        // res err
        if res < 0 {
            return Err(Error::last_os_error());
        }

        Ok(saddr.nl_pid)
    }

    fn bind(&mut self) -> Result<()> {
        let sock = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                self.proto,
            )
        };

        if sock < 0 {
            return Err(Error::last_os_error());
        }
        self.fd = sock;

        // bind
        let mut saddr = self.sockaddr();
        let res = unsafe {
            libc::bind(
                self.fd,
                mem::transmute(&mut saddr),
                mem::size_of::<libc::sockaddr_nl>() as u32,
            )
        };
        if res < 0 {
            return Err(Error::last_os_error());
        }
        return Ok(());
    }

    fn send(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut saddr = self.sockaddr();
        let len = buf.len();
        let res = unsafe {
            libc::sendto(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                len,
                0, // flags
                mem::transmute(&mut saddr),
                mem::size_of::<libc::sockaddr_nl>() as u32,
            )
        };

        if res < 0 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<Vec<super::NetlinkMessage>> {
        if self.fd <= 0 {
            return Err(Error::new(ErrorKind::NotConnected, "not connected"));
        }

        let mut buf: Vec<u8> = Vec::new();
        // TODO: peek instead of giant buffer
        buf.resize(RECEIVE_BUFFER_SIZE, 0);

        let len = buf.len();
        let res = unsafe {
            libc::recv(
                self.fd,
                buf.as_mut_slice().as_mut_ptr() as *mut libc::c_void,
                len,
                0, //flags
            )
        };
        if res < 0 {
            return Err(Error::last_os_error());
        }
        let res: usize = res as usize;

        // the slice is (probably) too big; resize to the returned length.
        if res < super::NetlinkHeader::size() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "netlink message too short",
            ));
        }
        buf.truncate(res);

        let msgs = super::NetlinkMessage::from_bytes(&buf)?;

        return Ok(msgs);
    }

    pub fn exec(
        &mut self,
        request: &mut super::NetlinkMessage,
        resp_typ: Option<u16>,
    ) -> Result<Vec<super::NetlinkMessage>> {
        // TODO: make this atomic and don't take a mut self.
        request.header.seq = self.next_seq;
        self.next_seq += 1;

        // send the message
        let mut b = request.to_bytes();
        self.send(&mut b)?;

        // Loop received messages
        let pid = self.pid()?;

        let mut out: Vec<super::NetlinkMessage> = vec![];
        loop {
            let mut resps = self.recv()?;
            for resp in resps.drain(0..) {
                // Validate response:

                // seq no matches
                if resp.header.seq != request.header.seq {
                    // We don't currently support shared sockets
                    return Err(Error::new(ErrorKind::InvalidData, "Incorrect seq number"));
                }

                // port id matches
                if resp.header.pid != pid {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Got incorrect responding port ID.",
                    ));
                }

                // Did the kernel return an error?
                // The errno is just the next 4 bytes
                if (resp.header.typ as i32) == libc::NLMSG_ERROR {
                    if resp.data.len() < 4 {
                        return Err(Error::new(
                            ErrorKind::UnexpectedEof,
                            "Error message too short",
                        ));
                    }
                    // TODO: rust 1.32 has proper byte order stuff, without
                    // needing a separate crate.
                    let mut rdr = Cursor::new(resp.data);
                    let errno: u32 = rdr.read_u32::<NativeEndian>().unwrap();
                    if errno == 0 {
                        return Ok(out);
                    }
                    return Err(Error::from_raw_os_error(-(errno as i32)));
                }

                // have we reached the end?
                if (resp.header.typ as i32) == libc::NLMSG_DONE {
                    return Ok(out);
                }

                // If we know which type of message we want, skip those
                // that don't match
                match resp_typ {
                    Some(typ) => {
                        if resp.header.typ != typ {
                            continue;
                        }
                    }
                    None => {}
                }

                // If we've gotten this far, the message is meant for us.
                // Add it to the result
                let respflags = resp.header.flags;
                out.push(resp);

                // If this isn't a mutipart message, we're done.
                if (respflags as i32) & libc::NLM_F_MULTI == 0 {
                    return Ok(out);
                }
            }
        }
    }
}

impl Drop for NetlinkSocket {
    fn drop(&mut self) {
        if self.fd > 0 {
            unsafe { libc::close(self.fd) };
            self.fd = 0;
        }
    }
}
