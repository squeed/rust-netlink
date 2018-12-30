#![allow(dead_code)]
/*
 * Implementation of the netlink protocol.
 */
/// proto implements the netlink protocol and socket
///
///
pub use self::packet::{NetlinkHeader, NetlinkMessage};

pub mod conn;
mod packet;
