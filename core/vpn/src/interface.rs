use super::device::CaptureDevice;
use managed::{ManagedMap, ManagedSlice};
use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache, Route, Routes};
use smoltcp::wire::{EthernetAddress, IpCidr};
use std::collections::BTreeMap;

pub type CaptureInterface<'a> = EthernetInterface<'a, CaptureDevice>;

pub fn default_iface<'a>() -> CaptureInterface<'a> {
    let ethernet_addr = EthernetAddress(rand::random());
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let addrs = Vec::new();
    let routes = Routes::new(BTreeMap::new());

    EthernetInterfaceBuilder::new(CaptureDevice::default())
        .ethernet_addr(ethernet_addr)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(addrs)
        .routes(routes)
        .finalize()
}

pub fn add_iface_address(iface: &mut CaptureInterface, address: IpCidr) {
    iface.update_ip_addrs(|addrs| match addrs {
        ManagedSlice::Owned(ref mut vec) => vec.push(address),
        ManagedSlice::Borrowed(ref slice) => {
            let mut vec = slice.to_vec();
            vec.push(address);
            *addrs = vec.into();
        }
    });
}

pub fn add_iface_route(iface: &mut CaptureInterface, net_ip: IpCidr, route: Route) {
    iface.routes_mut().update(|routes| match routes {
        ManagedMap::Owned(ref mut map) => {
            map.insert(net_ip, route);
        }
        ManagedMap::Borrowed(ref map) => {
            let mut map: BTreeMap<IpCidr, Route> =
                map.iter().filter_map(|e| (*e).clone()).collect();
            map.insert(net_ip, route);
            *routes = map.into();
        }
    });
}
