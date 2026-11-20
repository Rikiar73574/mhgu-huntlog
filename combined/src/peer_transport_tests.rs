//! Host-only stand-in for Forge's ABI. Compiles and drives the production transport.
//! No init/deinit functions exist here: any production dependency on either fails compilation.
extern crate self as forge;
#[path = "peer.rs"]
mod peer;
#[path = "peer_transport.rs"]
mod peer_transport;
use peer::{Frame, MAX_PACKET};
use peer_transport::Transport;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
};
static SERIAL: Mutex<()> = Mutex::new(());
static NET: Mutex<Network> = Mutex::new(Network::new());
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fail {
    CreateFirst,
    CreateSecond,
    Bind,
    SetOpt,
    Connect,
}
struct Endpoint {
    node: u8,
    bound: Option<sys::socket::SockAddrIn>,
    remote: Option<sys::socket::SockAddrIn>,
    broadcast: bool,
    queue: VecDeque<Vec<u8>>,
}
struct Network {
    fds: BTreeMap<i32, Endpoint>,
    next: i32,
    creates: usize,
    destroyed: Vec<i32>,
    fail: Option<Fail>,
    receives: usize,
    sends: usize,
}
impl Network {
    const fn new() -> Self {
        Self {
            fds: BTreeMap::new(),
            next: 100,
            creates: 0,
            destroyed: Vec::new(),
            fail: None,
            receives: 0,
            sends: 0,
        }
    }
    fn reset(&mut self, fail: Option<Fail>) {
        *self = Self::new();
        self.fail = fail;
        self.fds.insert(
            7,
            Endpoint {
                node: 255,
                bound: None,
                remote: None,
                broadcast: false,
                queue: VecDeque::new(),
            },
        );
    }
    fn fails(&mut self, kind: Fail) -> bool {
        if self.fail == Some(kind) {
            self.fail = None;
            true
        } else {
            false
        }
    }
}
pub mod sys {
    pub mod socket {
        use super::super::{Endpoint, Fail, NET};
        use std::{
            collections::VecDeque,
            ffi::{c_char, c_void},
            mem::size_of,
        };
        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        pub struct Socket {
            pub fd: i32,
        }
        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        pub struct SockAddrIn {
            pub sin_len: u8,
            pub sin_family: u8,
            pub sin_port: u16,
            pub sin_addr: [u8; 4],
            pub sin_zero: [c_char; 8],
        }
        unsafe fn addr(a: *const SockAddrIn, len: usize) -> SockAddrIn {
            assert_eq!(size_of::<SockAddrIn>(), 16);
            assert_eq!(len, 16);
            let a = unsafe { &*a };
            assert_eq!((a.sin_len, a.sin_family), (16, 2));
            assert_eq!(a.sin_port.to_ne_bytes(), 52689u16.to_be_bytes());
            assert_eq!(a.sin_zero, [0; 8]);
            *a
        }
        pub unsafe fn forge_socket_create(domain: i32, ttype: i32, protocol: i32) -> Socket {
            assert_eq!((domain, ttype, protocol), (2, 2, 0));
            let mut n = NET.lock().unwrap();
            n.creates += 1;
            let fail = if n.creates == 1 {
                Fail::CreateFirst
            } else {
                Fail::CreateSecond
            };
            if n.creates <= 2 && n.fails(fail) {
                return Socket { fd: -1 };
            }
            let fd = n.next;
            n.next += 1;
            let node = ((n.creates - 1) / 2) as u8;
            n.fds.insert(
                fd,
                Endpoint {
                    node,
                    bound: None,
                    remote: None,
                    broadcast: false,
                    queue: VecDeque::new(),
                },
            );
            Socket { fd }
        }
        pub unsafe fn forge_socket_destroy(socket: *mut Socket) {
            let s = unsafe { &mut *socket };
            if s.fd < 0 {
                return;
            }
            assert_ne!(s.fd, 7, "game socket closed");
            let mut n = NET.lock().unwrap();
            assert!(n.fds.remove(&s.fd).is_some(), "double close");
            n.destroyed.push(s.fd);
            s.fd = -1;
        }
        pub unsafe fn forge_socket_bind(
            socket: *const Socket,
            a: *const SockAddrIn,
            len: usize,
        ) -> i32 {
            let a = unsafe { addr(a, len) };
            assert_eq!(a.sin_addr, [0; 4]);
            let mut n = NET.lock().unwrap();
            if n.fails(Fail::Bind) {
                return -1;
            }
            n.fds.get_mut(&unsafe { (*socket).fd }).unwrap().bound = Some(a);
            0
        }
        pub unsafe fn forge_socket_connect(
            socket: *const Socket,
            a: *const SockAddrIn,
            len: usize,
        ) -> i32 {
            let a = unsafe { addr(a, len) };
            assert_eq!(a.sin_addr, [255; 4]);
            let mut n = NET.lock().unwrap();
            if n.fails(Fail::Connect) {
                return -1;
            }
            let e = n.fds.get_mut(&unsafe { (*socket).fd }).unwrap();
            assert!(e.broadcast);
            e.remote = Some(a);
            0
        }
        #[allow(non_snake_case)]
        pub unsafe fn forge_socket_setSockOpt(
            socket: *const Socket,
            level: i32,
            name: i32,
            val: *const c_void,
            len: u32,
        ) -> i32 {
            assert_eq!((level, name, len), (0xffff, 0x20, 4));
            assert_eq!(unsafe { *(val as *const i32) }, 1);
            let mut n = NET.lock().unwrap();
            if n.fails(Fail::SetOpt) {
                return -1;
            }
            n.fds.get_mut(&unsafe { (*socket).fd }).unwrap().broadcast = true;
            0
        }
        pub unsafe fn forge_socket_send(
            socket: *const Socket,
            buf: *const c_void,
            len: usize,
            flags: i32,
        ) -> isize {
            assert_eq!(flags, 0x80, "blocking send");
            assert!(len <= super::super::MAX_PACKET);
            let bytes = unsafe { std::slice::from_raw_parts(buf as *const u8, len) };
            let mut n = NET.lock().unwrap();
            n.sends += 1;
            let e = &n.fds[&unsafe { (*socket).fd }];
            assert!(e.bound.is_none(), "TX must be separate from RX");
            let remote = e.remote.unwrap();
            let node = e.node;
            let source = [10, 0, 0, node + 1];
            let source_port = (45000 + unsafe { (*socket).fd } as u16).to_be();
            assert_eq!(remote.sin_addr, [255; 4]);
            for receiver in n.fds.values_mut() {
                if receiver.node == node {
                    continue;
                }
                let Some(bound) = receiver.bound else {
                    continue;
                };
                if bound.sin_port != remote.sin_port {
                    continue;
                }
                if receiver
                    .remote
                    .is_some_and(|r| r.sin_addr != source || r.sin_port != source_port)
                {
                    continue;
                }
                receiver.queue.push_back(bytes.to_vec());
            }
            len as isize
        }
        pub unsafe fn forge_socket_recv(
            socket: *const Socket,
            buf: *mut c_void,
            len: usize,
            flags: i32,
        ) -> isize {
            assert_eq!(flags, 0x80, "blocking receive");
            assert_eq!(
                len,
                super::super::MAX_PACKET + 1,
                "must detect oversize datagrams"
            );
            let mut n = NET.lock().unwrap();
            n.receives += 1;
            let e = n.fds.get_mut(&unsafe { (*socket).fd }).unwrap();
            assert!(e.bound.is_some());
            let Some(bytes) = e.queue.pop_front() else {
                return -1;
            };
            let copied = len.min(bytes.len());
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copied) };
            copied as isize
        }
    }
}
fn frame(slot: u8, sequence: u32) -> Frame {
    let mut f = Frame::EMPTY;
    f.quest = 42;
    f.elapsed_ms = 10_000;
    f.epoch = 100 + slot as u64;
    f.sequence = sequence;
    f.slot = slot;
    f.meter_count = 1;
    f.meters[0].network = u16::MAX;
    f.meters[0].count = 1;
    f.meters[0].elapsed_ms = 10_000;
    f.meters[0].rows[0].damage = 120;
    f.meters[0].rows[0].hits = 2;
    f
}
#[test]
fn endpoints_exchange_snapshots_without_adding_damage_and_keep_rx_unconnected() {
    let _serial = SERIAL.lock().unwrap();
    NET.lock().unwrap().reset(None);
    let mut a = Transport::new();
    let mut b = Transport::new();
    a.tick(0.0);
    b.tick(0.0);
    {
        let n = NET.lock().unwrap();
        assert_eq!(n.fds.len(), 5);
        for rx in [100, 102] {
            let e = &n.fds[&rx];
            assert!(e.bound.is_some());
            assert!(e.remote.is_none());
        }
        for tx in [101, 103] {
            let e = &n.fds[&tx];
            assert!(e.bound.is_none());
            assert!(e.remote.is_some());
        }
    }
    assert!(a.send(&frame(0, 1)));
    let got = b.receive().unwrap();
    assert_eq!(got, frame(0, 1));
    let mut cache = peer::Cache::new();
    assert!(cache.accept(got, 42, 10_000));
    assert!(a.send(&frame(0, 2)));
    assert!(cache.accept(b.receive().unwrap(), 42, 10_000));
    assert_eq!(cache.leader().unwrap().meters[0].rows[0].damage, 120);
    assert!(b.send(&frame(1, 1)));
    assert_eq!(a.receive(), Some(frame(1, 1)));
    // Model the exact connected-UDP broadcast filter regression: real unicast source
    // addresses fail a broadcast remote filter. Unconnected RX above passed both ways.
    {
        let mut n = NET.lock().unwrap();
        n.fds.get_mut(&102).unwrap().remote = Some(sys::socket::SockAddrIn {
            sin_len: 16,
            sin_family: 2,
            sin_port: 52689u16.to_be(),
            sin_addr: [255; 4],
            sin_zero: [0; 8],
        });
    }
    assert!(a.send(&frame(0, 3)));
    assert!(b.receive().is_none());
    a.close();
    b.close();
    let n = NET.lock().unwrap();
    assert_eq!(n.fds.len(), 1);
    assert!(n.fds.contains_key(&7));
    assert_eq!(n.destroyed.len(), 4);
}
#[test]
fn malformed_truncated_oversize_and_empty_datagrams_consume_the_eight_packet_budget() {
    let _serial = SERIAL.lock().unwrap();
    NET.lock().unwrap().reset(None);
    let mut t = Transport::new();
    t.tick(0.0);
    let mut good = [0; MAX_PACKET];
    let len = peer::encode(&frame(1, 1), &mut good).unwrap();
    {
        let mut n = NET.lock().unwrap();
        let q = &mut n.fds.get_mut(&100).unwrap().queue;
        q.push_back(vec![]);
        q.push_back(good[..10].to_vec());
        q.push_back(vec![0; MAX_PACKET + 100]);
        q.push_back(vec![0; MAX_PACKET + 1]);
        q.push_back(vec![255; len]);
        q.push_back(vec![1]);
        q.push_back(vec![2]);
        q.push_back(vec![3]);
        q.push_back(good[..len].to_vec());
    }
    for _ in 0..20 {
        assert!(t.receive().is_none());
    }
    assert_eq!(NET.lock().unwrap().receives, 8);
    t.tick(0.25);
    assert_eq!(t.receive(), Some(frame(1, 1)));
    assert_eq!(NET.lock().unwrap().receives, 9);
    t.close();
    assert!(t.receive().is_none());
}
#[test]
fn partial_initialization_failures_close_only_owned_sockets_and_retry_after_five_seconds() {
    let _serial = SERIAL.lock().unwrap();
    for failure in [
        Fail::CreateFirst,
        Fail::CreateSecond,
        Fail::Bind,
        Fail::SetOpt,
        Fail::Connect,
    ] {
        NET.lock().unwrap().reset(Some(failure));
        let mut t = Transport::new();
        t.tick(0.0);
        assert!(!t.send(&frame(0, 1)));
        assert!(t.receive().is_none());
        {
            let n = NET.lock().unwrap();
            assert_eq!(n.creates, 2, "{failure:?}");
            assert_eq!(n.fds.len(), 1, "{failure:?}");
            assert!(n.fds.contains_key(&7));
            assert_eq!(
                n.destroyed.len(),
                if matches!(failure, Fail::CreateFirst | Fail::CreateSecond) {
                    1
                } else {
                    2
                }
            );
        }
        t.tick(4.0);
        t.tick(0.9);
        assert_eq!(NET.lock().unwrap().creates, 2);
        t.tick(0.2);
        assert_eq!(NET.lock().unwrap().creates, 4);
        assert!(t.send(&frame(0, 1)));
        t.close();
        t.close();
        assert_eq!(NET.lock().unwrap().fds.len(), 1);
    }
}
