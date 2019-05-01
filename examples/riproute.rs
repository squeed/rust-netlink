use netlink::proto::conn::NetlinkSocket;
use netlink::uapi;
use std::io::Result;

fn main() {
    let mut sock = NetlinkSocket::new(uapi::NETLINK_ROUTE as i32).unwrap();

    let res = netlink::hl::iface::link_list(&mut sock);
    match res {
        Err(e) => {
            println!("error: {}", e);
            return;
        }
        Ok(links) => {
            for link in links {
                println!("{:?}", &link);
                println!("{}: {}", link.index, link.name.unwrap().to_str().unwrap());
            }
        }
    }
}
