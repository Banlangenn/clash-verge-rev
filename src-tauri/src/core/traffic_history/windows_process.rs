use super::normalize_process;
use network_interface::{NetworkInterface, NetworkInterfaceConfig as _};
use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    mem::{align_of, size_of},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};
use tauri_plugin_mihomo::models::{ConnectionMetaData, Network};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, NO_ERROR},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, MIB_UDP6ROW_OWNER_PID,
        MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
    System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
};

type TcpEndpoint = (IpAddr, u16, u16);
type UdpEndpoint = (IpAddr, u16);
type Owners<Endpoint> = HashMap<Endpoint, Option<u32>>;
type ReadOwnerTable = unsafe extern "system" fn(*mut c_void, *mut u32, i32, u32, i32, u32) -> u32;

#[derive(Default)]
pub(super) struct SocketOwners {
    tcp: Option<Owners<TcpEndpoint>>,
    udp: Option<Owners<UdpEndpoint>>,
    local_ips: Option<HashSet<IpAddr>>,
    names: HashMap<u32, String>,
}

impl SocketOwners {
    pub(super) fn resolve(&mut self, metadata: &ConnectionMetaData) -> String {
        let ip = metadata
            .source_ip
            .split('%')
            .next()
            .unwrap_or_default()
            .parse::<IpAddr>();
        let source_port = metadata.source_port.parse::<u16>();
        let (Ok(ip), Ok(source_port)) = (ip, source_port) else {
            return String::new();
        };
        let owner = match metadata.network {
            Network::TCP => metadata
                .destination_port
                .parse::<u16>()
                .ok()
                .and_then(|destination_port| {
                    self.tcp
                        .get_or_insert_with(read_tcp_owners)
                        .get(&(ip, source_port, destination_port))
                        .copied()
                        .flatten()
                }),
            Network::UDP => {
                let ip = ip.to_canonical();
                udp_owner(self.udp.get_or_insert_with(read_udp_owners), ip, source_port)
                    .filter(|_| ip.is_loopback() || self.local_ips.get_or_insert_with(read_local_ips).contains(&ip))
            }
            Network::ALLNet | Network::Unknown(_) => None,
        };
        let Some(owner) = owner else {
            return String::new();
        };
        self.names
            .entry(owner)
            .or_insert_with(|| read_process_name(owner))
            .clone()
    }
}

fn record_owner<Endpoint: Eq + std::hash::Hash>(owners: &mut Owners<Endpoint>, endpoint: Endpoint, owner: u32) {
    if owner == 0 {
        return;
    }
    owners
        .entry(endpoint)
        .and_modify(|previous| {
            if *previous != Some(owner) {
                *previous = None;
            }
        })
        .or_insert(Some(owner));
}

fn read_tcp_owners() -> Owners<TcpEndpoint> {
    let mut owners = HashMap::new();
    let mut insert = |ip: IpAddr, source_port: u32, destination_port: u32, owner: u32, state: u32| {
        if !matches!(state, 1 | 2 | 11 | 12) {
            let endpoint = (ip, decode_port(source_port), decode_port(destination_port));
            record_owner(&mut owners, endpoint, owner);
        }
    };
    for row in read_table::<MIB_TCPROW_OWNER_PID>(GetExtendedTcpTable, AF_INET.into(), TCP_TABLE_OWNER_PID_ALL) {
        insert(
            Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()).into(),
            row.dwLocalPort,
            row.dwRemotePort,
            row.dwOwningPid,
            row.dwState,
        );
    }
    for row in read_table::<MIB_TCP6ROW_OWNER_PID>(GetExtendedTcpTable, AF_INET6.into(), TCP_TABLE_OWNER_PID_ALL) {
        insert(
            Ipv6Addr::from(row.ucLocalAddr).into(),
            row.dwLocalPort,
            row.dwRemotePort,
            row.dwOwningPid,
            row.dwState,
        );
    }
    owners
}

fn read_udp_owners() -> Owners<UdpEndpoint> {
    let mut owners = HashMap::new();
    for row in read_table::<MIB_UDPROW_OWNER_PID>(GetExtendedUdpTable, AF_INET.into(), UDP_TABLE_OWNER_PID) {
        let ip = Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()).into();
        record_owner(&mut owners, (ip, decode_port(row.dwLocalPort)), row.dwOwningPid);
    }
    for row in read_table::<MIB_UDP6ROW_OWNER_PID>(GetExtendedUdpTable, AF_INET6.into(), UDP_TABLE_OWNER_PID) {
        let ip = IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)).to_canonical();
        record_owner(&mut owners, (ip, decode_port(row.dwLocalPort)), row.dwOwningPid);
    }
    owners
}

fn read_local_ips() -> HashSet<IpAddr> {
    NetworkInterface::show()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|interface| interface.addr.into_iter().map(|address| address.ip().to_canonical()))
        .collect()
}

fn udp_owner(owners: &Owners<UdpEndpoint>, ip: IpAddr, port: u16) -> Option<u32> {
    let wildcard = match ip {
        IpAddr::V4(_) => Ipv4Addr::UNSPECIFIED.into(),
        IpAddr::V6(_) => Ipv6Addr::UNSPECIFIED.into(),
    };
    match (owners.get(&(ip, port)), owners.get(&(wildcard, port))) {
        (Some(exact), Some(wildcard)) if exact != wildcard => None,
        (Some(owner), _) | (None, Some(owner)) => *owner,
        (None, None) => None,
    }
}

const fn decode_port(port: u32) -> u16 {
    u16::from_be(port as u16)
}

fn read_table<Row: Copy>(read: ReadOwnerTable, family: u32, class: i32) -> Vec<Row> {
    let mut size_bytes = 32_768_u32;
    let rows_offset = size_of::<u32>().next_multiple_of(align_of::<Row>());
    for _ in 0..2 {
        let mut buffer = vec![0_u32; (size_bytes as usize).div_ceil(size_of::<u32>())];
        // The owner-PID tables start with a DWORD count followed by the aligned rows.
        let result = unsafe { read(buffer.as_mut_ptr().cast(), &raw mut size_bytes, 0, family, class, 0) };
        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if result != NO_ERROR || buffer.is_empty() {
            return Vec::new();
        }
        let max_rows = (buffer.len() * size_of::<u32>()).saturating_sub(rows_offset) / size_of::<Row>();
        let count = (buffer[0] as usize).min(max_rows);
        // The TCP and UDP owner-PID row fields are all integers.
        return (0..count)
            .map(|index| unsafe {
                buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(rows_offset)
                    .cast::<Row>()
                    .add(index)
                    .read_unaligned()
            })
            .collect();
    }
    Vec::new()
}

fn read_process_name(owner: u32) -> String {
    // Query only the process image basename, without opening a handle for memory or modification.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, owner) };
    if process.is_null() {
        return String::new();
    }
    let mut path = [0_u16; 4096];
    let mut len = path.len() as u32;
    let success = unsafe {
        let result = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &raw mut len);
        CloseHandle(process);
        result != 0
    };
    if success {
        normalize_process(&String::from_utf16_lossy(&path[..len as usize]))
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    #[test]
    fn windows_resolves_ipv4_and_ipv6_udp_wildcard_bindings() -> std::io::Result<()> {
        let ipv4 = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        let ipv6 = UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))?;
        let expected = normalize_process(&std::env::current_exe()?.to_string_lossy());
        let mut owners = SocketOwners::default();
        for (ip, socket) in [
            (IpAddr::V4(Ipv4Addr::LOCALHOST), ipv4),
            (IpAddr::V6(Ipv6Addr::LOCALHOST), ipv6),
        ] {
            let metadata = ConnectionMetaData {
                network: Network::UDP,
                source_ip: ip.to_string(),
                source_port: socket.local_addr()?.port().to_string(),
                ..ConnectionMetaData::default()
            };
            assert_eq!(owners.resolve(&metadata), expected);
        }
        assert!(owners.tcp.is_none());
        Ok(())
    }

    #[test]
    fn udp_does_not_assign_lan_clients_to_local_wildcard_sockets() {
        let wildcard = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let mut owners = SocketOwners {
            udp: Some(HashMap::from([((wildcard, 6000), Some(12))])),
            local_ips: Some(HashSet::new()),
            names: HashMap::from([(12, "local.exe".to_owned())]),
            ..SocketOwners::default()
        };
        let metadata = ConnectionMetaData {
            network: Network::UDP,
            source_ip: "192.0.2.1".to_owned(),
            source_port: "6000".to_owned(),
            ..ConnectionMetaData::default()
        };
        assert!(owners.resolve(&metadata).is_empty());
    }

    #[test]
    fn udp_leaves_shared_ports_unattributed() {
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let wildcard = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        let mut owners = HashMap::new();
        assert_eq!(udp_owner(&owners, ip, 6000), None);
        record_owner(&mut owners, (ip, 6000), 12);
        assert_eq!(udp_owner(&owners, ip, 6000), Some(12));
        record_owner(&mut owners, (wildcard, 6000), 12);
        assert_eq!(udp_owner(&owners, ip, 6000), Some(12));
        record_owner(&mut owners, (wildcard, 6000), 13);
        assert_eq!(udp_owner(&owners, ip, 6000), None);
    }
}
