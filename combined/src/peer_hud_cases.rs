use super::*;
use std::{collections::BTreeMap, sync::Mutex};
static TEST_LOCK: Mutex<()> = Mutex::new(());
const BASE: u32 = 0x0020_0000;
const OBJECT: u32 = 0x1300_0000;
#[derive(Default)]
struct Mem(BTreeMap<u32, u8>);
impl Mem {
    fn put(&mut self, a: u32, v: u32, n: usize) {
        for (i, b) in v.to_le_bytes()[..n].iter().enumerate() {
            self.0.insert(a + i as u32, *b);
        }
    }
    fn party(slot: u8, mask: u8) -> Self {
        let mut m = Self::default();
        m.put(BASE + 0x188ca48, 0x10000000, 4);
        m.put(0x10000030, slot as u32, 4);
        m.put(BASE + 0x1885b58, 0x11000000, 4);
        for i in 0..4 {
            m.put(
                0x1100001c + i * 4,
                if mask & (1 << i) != 0 {
                    0x12000000 + i * 4
                } else {
                    0
                },
                4,
            );
        }
        m
    }
    fn owner(&mut self, object: u32, local: bool, slot: u8) {
        self.put(object + 0x7420, local as u32, 1);
        self.put(object + 0x7424, slot as u32, 1);
    }
}
impl Memory for Mem {
    fn u8(&self, a: u32) -> Option<u8> {
        self.0.get(&a).copied()
    }
    fn u16(&self, a: u32) -> Option<u16> {
        Some(u16::from_le_bytes([self.u8(a)?, self.u8(a + 1)?]))
    }
    fn u32(&self, a: u32) -> Option<u32> {
        Some(u32::from_le_bytes([
            self.u8(a)?,
            self.u8(a + 1)?,
            self.u8(a + 2)?,
            self.u8(a + 3)?,
        ]))
    }
}
fn snapshot(object: u32) -> monsters::Snapshot {
    let mut s = monsters::Snapshot::new();
    s.count = 1;
    s.entries[0] = monsters::Monster {
        object,
        network: 7,
        species: 60,
        hp: 100,
        ailments: [(1, 10), (2, 20), (3, 30), (0, 100), (22, 150), (31, 200)],
    };
    s
}
fn dps(damage: u64, elapsed: f32) -> DpsState {
    let mut d = DpsState::new();
    d.count = 2;
    d.elapsed = elapsed;
    d.total = damage + 30;
    d.active = true;
    d.in_quest = true;
    for i in 0..2 {
        d.owners[i] = OwnerDps {
            owner: OBJECT + i as u32,
            slot: i as u8,
            color: 3 + i as u8,
            weapon: 5 + i as u8,
            damage: if i == 0 { damage } else { 30 },
            hits: 2 + i as u32,
            local: i == 0,
            name: PlayerName {
                bytes: [b'A' + i as u8; 32],
                len: 1,
            },
        };
    }
    d
}
fn reset() {
    RUNTIME.with(|s| *s = Runtime::new());
    Transport::reset_bus();
    DPS.with(|d| *d = DpsState::new());
    MONSTER_DPS.with(|p| *p = crate::MonsterDps::new());
}
fn step(
    mem: &Mem,
    monsters: &mut monsters::Snapshot,
    run: u64,
    quest: u32,
    elapsed: f32,
    seconds: f32,
) {
    update(
        mem,
        BASE,
        Observation::Quest {
            id: quest,
            remaining: 0,
            elapsed,
            stage: 1,
        },
        run,
        seconds,
        monsters,
    );
}
fn remote(slot: u8, sequence: u32, damage: u64) -> peer::Frame {
    let mut f = peer::Frame {
        quest: 42,
        elapsed_ms: 10000,
        epoch: 100 + slot as u64,
        sequence,
        slot,
        meter_count: 2,
        status_count: 1,
        ..peer::Frame::EMPTY
    };
    f.meters[0] = export(u16::MAX, 0, &dps(damage, 10.));
    f.meters[1] = export(7, 60, &dps(damage / 2, 8.));
    f.statuses[0] = peer::Status {
        network: 7,
        species: 60,
        pairs: [(0, 100), (22, 150), (31, 200)],
    };
    f
}
fn inject(f: peer::Frame) {
    RUNTIME.with(|s| s.transport.inject(f));
}
#[test]
fn real_import_export_preserves_rows_elapsed_and_local_highlight() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let original = dps(120, 12.345);
    let wire = export(7, 60, &original);
    assert_eq!(
        (wire.network, wire.species, wire.elapsed_ms),
        (7, 60, 12345)
    );
    let mut received = DpsState::new();
    import(&mut received, &wire, 1);
    assert_eq!(
        (received.total, received.elapsed, received.count),
        (150, 12.345, 2)
    );
    for i in 0..2 {
        assert_eq!(received.owners[i].local, i == 1);
        assert_eq!(received.owners[i].damage, original.owners[i].damage);
        assert_eq!(received.owners[i].name.bytes, original.owners[i].name.bytes);
        assert_eq!(received.owners[i].color, original.owners[i].color);
        assert_eq!(received.owners[i].weapon, original.owners[i].weapon);
    }
}
#[test]
fn update_replaces_network_totals_and_never_changes_local_accounting() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mem = Mem::party(1, 3);
    let mut monsters = snapshot(OBJECT + 0x10000);
    DPS.with(|d| *d = dps(9, 3.));
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    inject(remote(0, 1, 120));
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    let a = meter(u16::MAX, 0).unwrap();
    assert_eq!((a.total, a.elapsed), (150, 10.));
    assert!(!a.owners[0].local && a.owners[1].local);
    inject(remote(0, 2, 120));
    step(&mem, &mut monsters, 1, 42, 10., 0.01);
    assert_eq!(meter(u16::MAX, 0).unwrap().total, 150);
    DPS.with(|d| assert_eq!((d.total, d.elapsed), (39, 3.)));
    assert_eq!(meter(7, 60).unwrap().total, 90);
    assert!(meter(7, 61).is_none());
}
#[test]
fn publish_uses_stable_monster_identity_and_only_actual_ai_owner() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut mem = Mem::party(1, 3);
    let mut monsters = snapshot(OBJECT);
    mem.owner(OBJECT, true, 0);
    DPS.with(|d| *d = dps(100, 10.));
    MONSTER_DPS.with(|p| {
        p.keys[0] = OBJECT;
        p.states[0] = dps(50, 5.);
    });
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    RUNTIME.with(|s| {
        let f = s.transport.sent.last().unwrap();
        assert_eq!(f.meter_count, 2);
        assert_eq!((f.meters[1].network, f.meters[1].species), (7, 60));
        assert_eq!(f.status_count, 0);
    });
    mem.owner(OBJECT, true, 1);
    step(&mem, &mut monsters, 1, 42, 10.25, 0.25);
    RUNTIME.with(|s| {
        let f = s.transport.sent.last().unwrap();
        assert_eq!(f.status_count, 1);
        assert_eq!(f.statuses[0].pairs[0], (0, 100));
    });
}
#[test]
fn guest_uses_only_ai_owner_report_and_hides_expired_thresholds() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut mem = Mem::party(1, 7);
    let mut monsters = snapshot(OBJECT);
    mem.owner(OBJECT, false, 2);
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    inject(remote(0, 1, 120));
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    assert_eq!(monsters.entries[0].ailments[3..], [(0, 0); 3]);
    inject(remote(2, 1, 400));
    step(&mem, &mut monsters, 1, 42, 10., 0.01);
    assert_eq!(
        monsters.entries[0].ailments[3..],
        [(0, 100), (22, 150), (31, 200)]
    );
    step(&mem, &mut monsters, 1, 42, 11.5, 1.5);
    assert_eq!(monsters.entries[0].ailments[3..], [(0, 0); 3]);
    assert_eq!(
        monsters.entries[0].ailments[..3],
        [(1, 10), (2, 20), (3, 30)]
    );
}
#[test]
fn local_ai_owner_keeps_native_status_values() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut mem = Mem::party(1, 3);
    let mut monsters = snapshot(OBJECT);
    mem.owner(OBJECT, true, 1);
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    inject(remote(0, 1, 120));
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    assert_eq!(
        monsters.entries[0].ailments[3..],
        [(0, 100), (22, 150), (31, 200)]
    );
}
#[test]
fn quest_and_run_changes_clear_network_meters() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mem = Mem::party(1, 3);
    let mut monsters = snapshot(OBJECT);
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    inject(remote(0, 1, 120));
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    assert!(meter(u16::MAX, 0).is_some());
    step(&mem, &mut monsters, 2, 43, 0., 0.);
    assert!(meter(u16::MAX, 0).is_none());
    let mut f = remote(0, 2, 120);
    f.quest = 43;
    f.elapsed_ms = 0;
    inject(f);
    step(&mem, &mut monsters, 2, 43, 0., 0.);
    assert!(meter(u16::MAX, 0).is_some());
    step(&mem, &mut monsters, 3, 43, 0., 0.);
    assert!(meter(u16::MAX, 0).is_none());
}
#[test]
fn expired_leader_yields_to_fresh_peer_and_duplicates_do_not_refresh_ttl() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mem = Mem::party(3, 13);
    let mut monsters = snapshot(OBJECT);
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    inject(remote(0, 1, 120));
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    assert_eq!(meter(u16::MAX, 0).unwrap().total, 150);
    inject(remote(2, 1, 400));
    step(&mem, &mut monsters, 1, 42, 11., 1.);
    inject(remote(0, 1, 120));
    step(&mem, &mut monsters, 1, 42, 11.51, 0.51);
    assert_eq!(meter(u16::MAX, 0).unwrap().total, 430);
    step(&mem, &mut monsters, 1, 42, 13.1, 1.59);
    assert!(meter(u16::MAX, 0).is_none());
}

struct Endpoint {
    runtime: Runtime,
    local: DpsState,
    pool: crate::MonsterDps,
    monsters: monsters::Snapshot,
    mem: Mem,
}
impl Endpoint {
    fn new(slot: u8, object: u32, damage: u64) -> Self {
        let mut mem = Mem::party(slot, 3);
        mem.owner(object, slot == 0, 0);
        let mut pool = crate::MonsterDps::new();
        pool.keys[0] = object;
        pool.states[0] = dps(damage / 2, 8.);
        Self {
            runtime: Runtime::new(),
            local: dps(damage, 10.),
            pool,
            monsters: snapshot(object),
            mem,
        }
    }
    fn tick(&mut self) {
        RUNTIME.with(|s| std::mem::swap(s, &mut self.runtime));
        DPS.with(|d| std::mem::swap(d, &mut self.local));
        MONSTER_DPS.with(|p| std::mem::swap(p, &mut self.pool));
        step(&self.mem, &mut self.monsters, 1, 42, 10., 0.25);
        MONSTER_DPS.with(|p| std::mem::swap(p, &mut self.pool));
        DPS.with(|d| std::mem::swap(d, &mut self.local));
        RUNTIME.with(|s| std::mem::swap(s, &mut self.runtime));
    }
    fn displayed(&mut self, network: u16, species: u16) -> DpsState {
        RUNTIME.with(|s| std::mem::swap(s, &mut self.runtime));
        let out = meter(network, species).unwrap();
        RUNTIME.with(|s| std::mem::swap(s, &mut self.runtime));
        out
    }
}
#[test]
fn two_actual_adapters_share_canonical_totals_with_distinct_object_pointers() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut host = Endpoint::new(0, OBJECT, 120);
    let mut guest = Endpoint::new(1, OBJECT + 0x10000, 900);
    for _ in 0..3 {
        host.tick();
        guest.tick();
    }
    let a = host.displayed(u16::MAX, 0);
    let b = guest.displayed(u16::MAX, 0);
    assert_eq!((a.total, a.elapsed), (150, 10.));
    assert_eq!((a.total, a.elapsed), (b.total, b.elapsed));
    assert!(a.owners[0].local && !a.owners[1].local);
    assert!(!b.owners[0].local && b.owners[1].local);
    assert_eq!(host.displayed(7, 60).total, 90);
    assert_eq!(guest.displayed(7, 60).total, 90);
    assert_eq!((host.local.total, guest.local.total), (150, 930));
    assert_eq!(
        guest.monsters.entries[0].ailments[3..],
        [(0, 100), (22, 150), (31, 200)]
    );
    for _ in 0..3 {
        host.tick();
        guest.tick();
    }
    assert_eq!(guest.displayed(u16::MAX, 0).total, 150);
}
#[test]
fn departed_leader_immediately_yields_to_current_party_member() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut mem = Mem::party(3, 13);
    let mut monsters = snapshot(OBJECT);
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    inject(remote(0, 1, 120));
    inject(remote(2, 1, 400));
    step(&mem, &mut monsters, 1, 42, 10., 0.01);
    assert_eq!(meter(u16::MAX, 0).unwrap().total, 150);
    mem = Mem::party(3, 12);
    step(&mem, &mut monsters, 1, 42, 10., 0.01);
    assert_eq!(meter(u16::MAX, 0).unwrap().total, 430);
}
#[test]
fn solo_native_ai_owner_is_untouched() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mut mem = Mem::party(0, 1);
    let mut monsters = snapshot(OBJECT);
    mem.owner(OBJECT, true, 0);
    step(&mem, &mut monsters, 1, 42, 10., 0.);
    assert!(meter(u16::MAX, 0).is_none());
    assert_eq!(
        monsters.entries[0].ailments[3..],
        [(0, 100), (22, 150), (31, 200)]
    );
}
#[test]
fn reconnect_in_same_run_keeps_wire_sequence_fresh_for_remote_cache() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reset();
    let mem = Mem::party(0, 3);
    let mut monsters = snapshot(OBJECT);
    step(&mem, &mut monsters, 44, 42, 10., 0.);
    let first = RUNTIME.with(|s| *s.transport.sent.last().unwrap());
    let mut other = peer::Cache::new();
    assert!(other.accept(first, 42, 10000));
    let alone = Mem::party(0, 1);
    step(&alone, &mut monsters, 44, 42, 10., 0.01);
    step(&mem, &mut monsters, 44, 42, 10., 0.01);
    let rejoined = RUNTIME.with(|s| *s.transport.sent.last().unwrap());
    assert_eq!(first.epoch, rejoined.epoch);
    assert!(rejoined.sequence.wrapping_sub(first.sequence) as i32 > 0);
    assert!(other.accept(rejoined, 42, 10000));
}

#[test]
fn reset_control_belongs_to_the_shared_reference() {
    let _lock = TEST_LOCK.lock().unwrap();
    reset();
    let mut monsters = snapshot(OBJECT);
    let mut mem = Mem::party(1, 3);
    mem.owner(OBJECT, false, 0);
    step(&mem, &mut monsters, 8, 42, 10.0, 0.0);
    inject(remote(0, 1, 70));
    step(&mem, &mut monsters, 8, 42, 10.0, 0.1);
    assert!(following());
    // Expiry restores the existing local control rather than leaving it locked.
    step(&mem, &mut monsters, 8, 42, 12.0, 2.0);
    assert!(!following());
    reset();
    mem = Mem::party(0, 3);
    mem.owner(OBJECT, true, 0);
    step(&mem, &mut monsters, 8, 42, 10.0, 0.0);
    inject(remote(1, 1, 70));
    step(&mem, &mut monsters, 8, 42, 10.0, 0.1);
    assert!(!following());
}
