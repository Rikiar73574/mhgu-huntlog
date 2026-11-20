//! Native integration harness: compile the production adapter against bounded game
//! stand-ins and an in-memory datagram transport; no game or Forge calls occur.
#![allow(dead_code)]
#[path = "peer.rs"]
mod peer;
#[path = "peer_hud.rs"]
mod peer_hud;
mod data {
    pub struct Shared<T>(std::sync::Mutex<T>);
    impl<T> Shared<T> {
        pub const fn new(value: T) -> Self {
            Self(std::sync::Mutex::new(value))
        }
        pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
            f(&mut self.0.lock().unwrap())
        }
    }
}
#[derive(Clone, Copy)]
struct PlayerName {
    bytes: [u8; 32],
    len: u8,
}
#[derive(Clone, Copy)]
struct OwnerDps {
    owner: u32,
    local: bool,
    damage: u64,
    hits: u32,
    slot: u8,
    color: u8,
    weapon: u8,
    name: PlayerName,
}
impl OwnerDps {
    const EMPTY: Self = Self {
        owner: 0,
        local: false,
        damage: 0,
        hits: 0,
        slot: 255,
        color: 255,
        weapon: 255,
        name: PlayerName {
            bytes: [0; 32],
            len: 0,
        },
    };
}
#[derive(Clone, Copy)]
struct DpsState {
    owners: [OwnerDps; 4],
    count: usize,
    total: u64,
    elapsed: f32,
    active: bool,
    in_quest: bool,
    samples: usize,
}
impl DpsState {
    const fn new() -> Self {
        Self {
            owners: [OwnerDps::EMPTY; 4],
            count: 0,
            total: 0,
            elapsed: 0.,
            active: false,
            in_quest: false,
            samples: 0,
        }
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn sample_history(&mut self) {
        self.samples += 1;
    }
}
#[derive(Clone, Copy)]
struct MonsterDps {
    keys: [u32; 16],
    states: [DpsState; 16],
}
impl MonsterDps {
    const fn new() -> Self {
        Self {
            keys: [0; 16],
            states: [DpsState::new(); 16],
        }
    }
}
static DPS: data::Shared<DpsState> = data::Shared::new(DpsState::new());
static MONSTER_DPS: data::Shared<MonsterDps> = data::Shared::new(MonsterDps::new());
mod target {
    pub trait Memory {
        fn u8(&self, a: u32) -> Option<u8>;
        fn u16(&self, a: u32) -> Option<u16>;
        fn u32(&self, a: u32) -> Option<u32>;
    }
}
mod quest {
    #[derive(Clone, Copy, PartialEq)]
    pub enum Observation {
        Unknown,
        Town,
        Quest {
            id: u32,
            remaining: u32,
            elapsed: f32,
            stage: u8,
        },
    }
}
mod monsters {
    #[derive(Clone, Copy)]
    pub struct Monster {
        pub object: u32,
        pub network: u16,
        pub species: u16,
        pub hp: i32,
        pub ailments: [(i16, i16); 6],
    }
    impl Monster {
        pub const EMPTY: Self = Self {
            object: 0,
            network: 0,
            species: 0,
            hp: 0,
            ailments: [(0, 0); 6],
        };
    }
    #[derive(Clone, Copy)]
    pub struct Snapshot {
        pub count: usize,
        pub entries: [Monster; 10],
    }
    impl Snapshot {
        pub const fn new() -> Self {
            Self {
                count: 0,
                entries: [Monster::EMPTY; 10],
            }
        }
    }
}
mod peer_transport {
    use super::peer;
    use std::{
        collections::{BTreeMap, VecDeque},
        sync::{LazyLock, Mutex},
    };
    struct Bus {
        next: usize,
        queues: BTreeMap<usize, VecDeque<Vec<u8>>>,
    }
    static BUS: LazyLock<Mutex<Bus>> = LazyLock::new(|| {
        Mutex::new(Bus {
            next: 0,
            queues: BTreeMap::new(),
        })
    });
    pub struct Transport {
        id: Option<usize>,
        pub sent: Vec<peer::Frame>,
    }
    impl Transport {
        pub const fn new() -> Self {
            Self {
                id: None,
                sent: Vec::new(),
            }
        }
        pub fn tick(&mut self, _seconds: f32) {
            if self.id.is_none() {
                let mut b = BUS.lock().unwrap();
                let id = b.next;
                b.next += 1;
                b.queues.insert(id, VecDeque::new());
                self.id = Some(id);
            }
        }
        pub fn close(&mut self) {
            if let Some(id) = self.id.take() {
                BUS.lock().unwrap().queues.remove(&id);
            }
        }
        pub fn send(&mut self, f: &peer::Frame) {
            let mut bytes = [0; peer::MAX_PACKET];
            let n = peer::encode(f, &mut bytes)
                .expect("published frame must pass actual wire validation");
            self.sent.push(*f);
            for (&id, q) in &mut BUS.lock().unwrap().queues {
                if Some(id) != self.id {
                    q.push_back(bytes[..n].to_vec());
                }
            }
        }
        pub fn receive(&mut self) -> Option<peer::Frame> {
            let bytes = BUS.lock().unwrap().queues.get_mut(&self.id?)?.pop_front()?;
            peer::decode(&bytes)
        }
        pub fn inject(&mut self, f: peer::Frame) {
            self.tick(0.);
            let mut bytes = [0; peer::MAX_PACKET];
            let n = peer::encode(&f, &mut bytes).unwrap();
            BUS.lock()
                .unwrap()
                .queues
                .get_mut(&self.id.unwrap())
                .unwrap()
                .push_back(bytes[..n].to_vec());
        }
        pub fn reset_bus() {
            let mut b = BUS.lock().unwrap();
            b.next = 0;
            b.queues.clear();
        }
    }
}
