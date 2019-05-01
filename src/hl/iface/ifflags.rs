use bitflags::bitflags;

bitflags! {
    pub struct IfFlags: u32 {
        /// interface is up
        const UP      =    0x1;
        /// broadcast address valid
        const BROADCAST =  0x2;
        /// turn on debugging
        const DEBUG    =   0x4;
        /// is a loopback net
        const LOOPBACK  =  0x8;
        /// interface is a p-p link
        const POINTOPOINT = 0x10;
        /// avoid use of trailers
        const NOTRAILERS = 0x20;
        /// interface RFC2863 OPER_UP
        const RUNNING   =  0x40;
        /// no ARP protocol
        const NOARP     =  0x80;
        /// receive all packets
        const PROMISC   =  0x100;
        /// receive all multicast packets
        const ALLMULTI  =  0x200;
        /// master of a load balancer
        const MASTER    =  0x400;
        /// slave of a load balancer
        const SLAVE     =  0x800;
        /// Supports multicast
        const MULTICAST =  0x1000;
        /// can set media type
        const PORTSEL   =  0x2000;
        /// auto media select active
        const AUTOMEDIA =  0x4000;
        /// dialup device with changing addresses
        const DYNAMIC   =  0x8000;
        /// driver signals L1 up
        const LOWER_UP  =  0x10000;
        /// driver signals dormant
        const DORMANT   =  0x20000;
        /// echo sent packets
        const ECHO      =  0x40000;
    }
}

impl std::default::Default for IfFlags {
    fn default() -> Self {
        IfFlags::empty()
    }
}
