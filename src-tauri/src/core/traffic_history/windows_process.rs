use super::normalize_process;
use std::{
    collections::HashMap,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr::null_mut,
};
use tauri_plugin_mihomo::models::ConnectionMetaData;
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, NO_ERROR},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
    System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
};

type Endpoint = (IpAddr, u16, u16);

#[derive(Default)]
pub(super) struct SocketOwners {
    owners: HashMap<Endpoint, Option<u32>>,
    names: HashMap<u32, String>,
}

impl SocketOwners {
    pub(super) fn read() -> Self {
        let mut table = Self::default();
        for row in read_table::<MIB_TCPROW_OWNER_PID>(AF_INET.into()) {
            table.insert(
                IpAddr::V4(Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes())),
                row.dwLocalPort,
                row.dwRemotePort,
                row.dwOwningPid,
                row.dwState,
            );
        }
        for row in read_table::<MIB_TCP6ROW_OWNER_PID>(AF_INET6.into()) {
            table.insert(
                IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)),
                row.dwLocalPort,
                row.dwRemotePort,
                row.dwOwningPid,
                row.dwState,
            );
        }
        table
    }

    fn insert(&mut self, ip: IpAddr, source_port: u32, destination_port: u32, owner: u32, state: u32) {
        if owner == 0 || matches!(state, 1 | 2 | 11 | 12) {
            return;
        }
        let endpoint = (
            ip,
            u16::from_be(source_port as u16),
            u16::from_be(destination_port as u16),
        );
        self.owners
            .entry(endpoint)
            .and_modify(|previous| {
                if *previous != Some(owner) {
                    *previous = None;
                }
            })
            .or_insert(Some(owner));
    }

    pub(super) fn resolve(&mut self, metadata: &ConnectionMetaData) -> String {
        let ip = metadata
            .source_ip
            .split('%')
            .next()
            .unwrap_or_default()
            .parse::<IpAddr>();
        let source_port = metadata.source_port.parse::<u16>();
        let destination_port = metadata.destination_port.parse::<u16>();
        let (Ok(ip), Ok(source_port), Ok(destination_port)) = (ip, source_port, destination_port) else {
            return String::new();
        };
        let Some(Some(owner)) = self.owners.get(&(ip, source_port, destination_port)) else {
            return String::new();
        };
        self.names
            .entry(*owner)
            .or_insert_with(|| read_process_name(*owner))
            .clone()
    }
}

fn read_table<Row: Copy>(family: u32) -> Vec<Row> {
    let mut size_bytes = 32_768_u32;
    for _ in 0..2 {
        let mut buffer = vec![0_u32; (size_bytes as usize).div_ceil(size_of::<u32>())];
        // This API writes one DWORD row count, followed by the selected owner-PID row type.
        let result = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr().cast(),
                &raw mut size_bytes,
                0,
                family,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };
        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if result != NO_ERROR || buffer.is_empty() {
            return Vec::new();
        }
        let max_rows = (buffer.len() - 1) * size_of::<u32>() / size_of::<Row>();
        let count = (buffer[0] as usize).min(max_rows);
        // Only MIB_TCPROW_OWNER_PID and MIB_TCP6ROW_OWNER_PID are used. Their fields are integers.
        return (0..count)
            .map(|index| unsafe { buffer.as_ptr().add(1).cast::<Row>().add(index).read_unaligned() })
            .collect();
    }
    Vec::new()
}

fn read_process_name(owner: u32) -> String {
    // Query only the process image basename, without opening a handle for memory or modification.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, owner) };
    if process == null_mut() {
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
