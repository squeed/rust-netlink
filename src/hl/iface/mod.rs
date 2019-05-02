/// iface: interface management
/// higher-level methods for dealing with interfaces.
///
/// Interfaces are represented by the kernel as a small set of properties and a
/// list of key-value pairs. Thus, everything is modeled as an Option, even though
/// the kernel will "always" send them when retrieving a link.
///
/// Likewise, when creating or updating a link, most fields are optional.
mod ifflags;
pub use self::ifflags::IfFlags;
use crate::proto::conn::NetlinkSocket;
use crate::proto::NetlinkMessage;
use crate::type_route::{IfInfoMsg, RtAttr};
use crate::uapi;
use crate::Serializable;
use std::default::Default;
use std::ffi::CString;
use std::io::{Error, ErrorKind, Result};

// First attempt: everything is a Maybe

/// LinkMsg is a representation the data the kernel will send and receive
/// describing a Link.
///
/// All fieds are optional except index, flags, and flags_change, because the
/// kernel doesn't actually require them
#[derive(Default, Debug)]
pub struct LinkMsg {
    pub index: i32,
    // TODO: implement a flags change type
    pub flags: IfFlags,
    pub flags_change: IfFlags,

    pub mtu: Option<u32>,
    pub tx_q_len: Option<u32>,
    pub name: Option<CString>,
    pub hadrware_addr: Option<Vec<u8>>,
    pub parent_index: Option<u32>,
    pub alias: Option<CString>,
    pub promisc: Option<i32>,
    pub kind: Option<CString>,
    pub master_index: Option<u32>,
    pub specific: LinkType,
}

#[derive(Debug)]
pub enum LinkType {
    Unknown,
    Bridge(Bridge),
    Dummy,
    Ifb,
    Veth(Veth),
    Vlan(Vlan),
}

impl Default for LinkType {
    fn default() -> Self {
        LinkType::Unknown
    }
}

impl LinkType {
    pub fn from_attrs(kind: &CString, rt_attrs: &Vec<RtAttr>) -> Result<LinkType> {
        let kind = kind.to_str().unwrap();
        let out = match kind {
            "bridge" => {
                let mut b: Bridge = Default::default();
                for rt_attr in rt_attrs.iter() {
                    match rt_attr.get_typ() as u32 {
                        uapi::IFLA_BR_VLAN_FILTERING => {
                            b.vlan_filtering = Some(rt_attr.as_bool().unwrap())
                        }
                        _ => {}
                    }
                }
                LinkType::Bridge(b)
            }
            "dummy" => LinkType::Dummy {},
            // TODO ifb

            // veth fields are create-only, oddly enough
            "veth" => LinkType::Veth(Default::default()),

            "vlan" => {
                let mut v: Vlan = Default::default();
                for rt_attr in rt_attrs.iter() {
                    match rt_attr.get_typ() as u32 {
                        uapi::IFLA_VLAN_ID => v.vlan_id = Some(rt_attr.as_u16().unwrap()),
                        _ => {}
                    };
                }
                LinkType::Vlan(v)
            }

            // unrecognized link type
            _ => LinkType::Unknown,
        };

        Ok(out)
    }
}

#[derive(Default, Debug)]
pub struct Vlan {
    vlan_id: Option<u16>,
}

#[derive(Default, Debug)]
pub struct Veth {
    // supported on create only
    peer_name: Option<i32>,
}

#[derive(Default, Debug)]
pub struct Bridge {
    vlan_filtering: Option<bool>,
}

impl LinkMsg {
    pub fn from_message(nlmsg: &NetlinkMessage) -> Result<LinkMsg> {
        // peel off ifinfo message
        let ifinfo = IfInfoMsg::from_bytes(&nlmsg.data)?;
        let attrs = RtAttr::from_bytes(&nlmsg.data[IfInfoMsg::size()..])?;
        let link = LinkMsg::from_attrs(&ifinfo, &attrs)?;
        Ok(link)
    }
    pub fn from_attrs(info: &IfInfoMsg, rt_attrs: &Vec<RtAttr>) -> Result<LinkMsg> {
        let mut out = LinkMsg {
            index: info.index,
            flags: IfFlags::from_bits_truncate(info.flags),
            flags_change: IfFlags::empty(),
            ..Default::default()
        };

        // todo: get rid of all of these unwraps
        // need to plumb through the result
        for rt_attr in rt_attrs.iter() {
            match rt_attr.get_typ() as u32 {
                uapi::IFLA_MTU => out.mtu = Some(rt_attr.as_u32().unwrap()),
                uapi::IFLA_IFNAME => out.name = Some(rt_attr.to_cstring().unwrap()),
                uapi::IFLA_TXQLEN => out.tx_q_len = Some(rt_attr.as_u32().unwrap()),
                // TODO: filter all-zero hwaddrs
                uapi::IFLA_ADDRESS => out.hadrware_addr = Some(rt_attr.data.to_owned()),
                uapi::IFLA_LINK => out.parent_index = Some(rt_attr.as_u32().unwrap()),
                uapi::IFLA_MASTER => out.master_index = Some(rt_attr.as_u32().unwrap()),
                uapi::IFLA_IFALIAS => out.alias = Some(rt_attr.to_cstring().unwrap()),
                // LINKINFO is just a nested list of more attributes
                uapi::IFLA_LINKINFO => {
                    let info_attrs = RtAttr::from_bytes(&rt_attr.data)?;
                    for info_attr in info_attrs.iter() {
                        match info_attr.get_typ() as u32 {
                            uapi::IFLA_INFO_KIND => {
                                out.kind = Some(info_attr.to_cstring().unwrap())
                            }
                            // TODO: kind-specific data, which is another array
                            // of rtattrs
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(out)
    }
}

pub fn link_list(sock: &mut NetlinkSocket) -> Result<Vec<LinkMsg>> {
    let mut req = NetlinkMessage::new(
        uapi::RTM_GETLINK as u16,
        (uapi::NLM_F_DUMP | uapi::NLM_F_REQUEST) as u16,
    );
    let msg = IfInfoMsg {
        family: uapi::AF_UNSPEC as u8,
        ..Default::default()
    };
    req.add_data(msg.to_bytes());

    let resp = sock.exec(&mut req, Some(uapi::RTM_NEWLINK as u16))?;

    let mut out = vec![];
    for nlmsg in resp {
        let link = LinkMsg::from_message(&nlmsg)?;
        out.push(link);
    }

    Ok(out)
}

pub fn link_get_by_index(sock: &mut NetlinkSocket, idx: i32) -> Result<LinkMsg> {
    let mut req = NetlinkMessage::new(
        uapi::RTM_GETLINK as u16,
        (uapi::NLM_F_ACK | uapi::NLM_F_REQUEST) as u16,
    );
    let msg = IfInfoMsg {
        family: uapi::AF_UNSPEC as u8,
        index: idx,
        ..Default::default()
    };
    req.add_data(msg.to_bytes());

    let resp = sock.exec(&mut req, Some(uapi::RTM_NEWLINK as u16))?;
    match resp.len() {
        0 => Err(Error::new(ErrorKind::NotFound, "link not found")),
        1 => LinkMsg::from_message(&resp[0]),
        _ => Err(Error::new(ErrorKind::Other, "too many links returned")),
    }
}
