//! Private HuntLog UDP channel. Uses the game's already initialized socket subsystem.
//! RX is deliberately unconnected: connecting it to broadcast filters real senders out.
use crate::peer::{self, Frame, MAX_PACKET};
use core::{ffi::c_void, mem::size_of};
use forge::sys::socket::{
    SockAddrIn, Socket, forge_socket_bind, forge_socket_connect, forge_socket_create,
    forge_socket_destroy, forge_socket_recv, forge_socket_send, forge_socket_setSockOpt,
};
const PORT: u16 = 52689;
const DONTWAIT: i32 = 0x80;
pub struct Transport {
    rx: Socket,
    tx: Socket,
    retry: f32,
    remaining: u8,
    buffer: [u8; MAX_PACKET + 1],
}
impl Transport {
    pub const fn new() -> Self {
        Self {
            rx: Socket { fd: -1 },
            tx: Socket { fd: -1 },
            retry: 0.0,
            remaining: 0,
            buffer: [0; MAX_PACKET + 1],
        }
    }
    /// One update permits at most eight datagrams, even if the caller keeps receiving.
    pub fn tick(&mut self, seconds: f32) {
        self.remaining = 8;
        let delta = if seconds.is_finite() && seconds >= 0.0 {
            seconds
        } else {
            0.0
        };
        self.retry = (self.retry - delta).max(0.0);
        if self.rx.fd >= 0 && self.tx.fd >= 0 || self.retry > 0.0 {
            return;
        }
        self.close();
        self.remaining = 8;
        unsafe {
            self.rx = forge_socket_create(2, 2, 0);
            self.tx = forge_socket_create(2, 2, 0);
            let mut addr = SockAddrIn {
                sin_len: 16,
                sin_family: 2,
                sin_port: PORT.to_be(),
                sin_addr: [0; 4],
                sin_zero: [0; 8],
            };
            // Switch SOL_SOCKET / SO_BROADCAST; the recomp's virtual socket layer accepts it.
            let enabled = 1i32;
            let ok = self.rx.fd >= 0
                && self.tx.fd >= 0
                && forge_socket_bind(&self.rx, &addr, size_of::<SockAddrIn>()) == 0
                && forge_socket_setSockOpt(
                    &self.tx,
                    0xffff,
                    0x20,
                    &enabled as *const _ as *const c_void,
                    4,
                ) == 0;
            addr.sin_addr = [255; 4];
            if !ok || forge_socket_connect(&self.tx, &addr, size_of::<SockAddrIn>()) != 0 {
                self.close();
                self.retry = 5.0;
            }
        }
    }
    /// Consumes one datagram. None also covers malformed packets; callers may continue
    /// their fixed eight-iteration drain instead of stopping at the first None.
    pub fn receive(&mut self) -> Option<Frame> {
        if self.rx.fd < 0 || self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let n = unsafe {
            forge_socket_recv(
                &self.rx,
                self.buffer.as_mut_ptr() as *mut c_void,
                self.buffer.len(),
                DONTWAIT,
            )
        };
        if n <= 0 || n as usize > MAX_PACKET {
            return None;
        }
        peer::decode(&self.buffer[..n as usize])
    }
    pub fn send(&mut self, frame: &Frame) -> bool {
        if self.tx.fd < 0 {
            return false;
        }
        let Some(n) = peer::encode(frame, &mut self.buffer[..MAX_PACKET]) else {
            return false;
        };
        unsafe {
            forge_socket_send(&self.tx, self.buffer.as_ptr() as *const c_void, n, DONTWAIT)
                == n as isize
        }
    }
    /// Drops queued packets and only our two descriptors, never the game's subsystem.
    pub fn close(&mut self) {
        unsafe {
            forge_socket_destroy(&mut self.rx);
            forge_socket_destroy(&mut self.tx);
        }
        self.retry = 0.0;
        self.remaining = 0;
    }
}
