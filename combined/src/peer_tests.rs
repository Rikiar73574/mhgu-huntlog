#[path = "peer.rs"]
mod peer;
use peer::*;
fn frame(slot: u8, sequence: u32) -> Frame {
    let mut f = Frame::EMPTY;
    f.quest = 42;
    f.elapsed_ms = 10_000;
    f.epoch = 123;
    f.slot = slot;
    f.sequence = sequence;
    f.meter_count = 1;
    f.meters[0].network = u16::MAX;
    f.meters[0].count = 1;
    f.meters[0].elapsed_ms = 10_000;
    f.meters[0].rows[0].name[0] = b'A';
    f.meters[0].rows[0].name_len = 1;
    f.meters[0].rows[0].damage = 120;
    f.meters[0].rows[0].hits = 2;
    f.status_count = 1;
    f.statuses[0].network = 7;
    f.statuses[0].species = 60;
    f.statuses[0].pairs = [(12, 100), (22, 150), (31, 200)];
    f
}
#[test]
fn roundtrip_and_exact_bounds() {
    let f = frame(1, 2);
    let mut b = [0; MAX_PACKET];
    let n = encode(&f, &mut b).unwrap();
    assert_eq!(decode(&b[..n]), Some(f));
    for len in 0..n {
        assert!(decode(&b[..len]).is_none());
    }
    assert!(encode(&f, &mut b[..n - 1]).is_none());
    b[n] = 0;
    assert!(decode(&b[..n + 1]).is_none());
}
#[test]
fn maximum_packet_is_bounded() {
    let mut f = frame(0, 1);
    f.meter_count = MAX_METERS as u8;
    for (i, m) in f.meters.iter_mut().enumerate() {
        m.network = i as u16;
        m.species = 1;
        m.count = 4;
        for (j, r) in m.rows.iter_mut().enumerate() {
            r.slot = j as u8;
        }
    }
    f.status_count = MAX_STATUSES as u8;
    for (i, s) in f.statuses.iter_mut().enumerate() {
        s.network = i as u16;
        s.species = 1;
    }
    let mut b = [0; MAX_PACKET];
    assert_eq!(encode(&f, &mut b), Some(MAX_PACKET));
    assert_eq!(decode(&b), Some(f));
}
#[test]
fn malformed_inputs_are_rejected() {
    let f = frame(0, 1);
    let mut b = [0; MAX_PACKET];
    let n = encode(&f, &mut b).unwrap();
    for (offset, value) in [(0, 0), (4, 99), (25, 4), (26, 255), (27, 255)] {
        let mut c = b;
        c[offset] = value;
        assert!(decode(&c[..n]).is_none());
    }
    let mut f = f;
    f.meters[0].rows[0].name[0] = 255;
    assert!(encode(&f, &mut b).is_none());
    f = frame(0, 1);
    f.meters[0].count = 2;
    assert!(encode(&f, &mut b).is_none());
    f = frame(0, 1);
    f.meter_count = 2;
    f.meters[1] = f.meters[0];
    assert!(encode(&f, &mut b).is_none());
    f = frame(0, 1);
    f.status_count = 2;
    f.statuses[1] = f.statuses[0];
    assert!(encode(&f, &mut b).is_none());
    f = frame(0, 1);
    f.statuses[0].pairs[0] = (-2, 100);
    assert!(encode(&f, &mut b).is_none());
}
#[test]
fn canonical_metrics_replace_without_adding_receipts() {
    let mut c = Cache::new();
    assert!(c.accept(frame(2, 1), 42, 10_000));
    assert!(c.accept(frame(0, 1), 42, 10_000));
    assert_eq!(c.leader().unwrap().slot, 0);
    assert_eq!(c.leader().unwrap().meters[0].rows[0].damage, 120);
    assert!(!c.accept(frame(0, 1), 42, 10_000));
    assert!(c.accept(frame(0, 2), 42, 10_000));
    assert_eq!(c.leader().unwrap().meters[0].rows[0].damage, 120);
    assert_eq!(c.owner(2).unwrap().statuses[0].pairs[1], (22, 150));
    assert!(c.owner(4).is_none());
}
#[test]
fn sequence_wrap_restart_and_expiry() {
    let mut c = Cache::new();
    assert!(c.accept(frame(0, u32::MAX), 42, 10_000));
    assert!(c.accept(frame(0, 0), 42, 10_000));
    assert!(!c.accept(frame(0, u32::MAX), 42, 10_000));
    c.advance(1.0);
    assert!(!c.accept(frame(0, 0), 42, 10_000));
    c.advance(0.51);
    assert!(c.leader().is_none());
    assert!(!c.accept(frame(0, 0), 42, 10_000));
    let mut f = frame(0, 0);
    f.epoch = 124;
    assert!(c.accept(f, 42, 10_000));
    assert!(!c.accept(frame(0, 1), 42, 10_000));
}
#[test]
fn quest_reset_rollback_and_compatibility() {
    let mut c = Cache::new();
    assert!(!c.accept(frame(0, 1), 43, 10_000));
    assert!(!c.accept(frame(0, 1), 42, 15_001));
    assert!(c.accept(frame(0, 1), 42, 10_000));
    assert!(c.accept(frame(2, 1), 42, 10_000));
    let mut f = frame(1, 1);
    f.elapsed_ms = 0;
    assert!(c.accept(f, 42, 0));
    assert!(c.owner(0).is_none());
    assert_eq!(c.leader().unwrap().slot, 1);
    c.reset();
    assert!(c.leader().is_none());
}
#[test]
fn native_unknown_rows_and_palico_weapon_are_preserved() {
    let mut f = frame(0, 1);
    let m = &mut f.meters[0];
    m.count = 3;
    m.rows[0].weapon = 15;
    m.rows[1].slot = 255;
    m.rows[1].color = 255;
    m.rows[1].weapon = 255;
    m.rows[2].slot = 255;
    let mut bytes = [0; MAX_PACKET];
    let n = encode(&f, &mut bytes).unwrap();
    assert_eq!(decode(&bytes[..n]), Some(f));
}
#[test]
fn unreasonable_values_and_wire_utf8_are_rejected() {
    let original = frame(0, 1);
    let mut bytes = [0; MAX_PACKET];
    for f in [
        Frame {
            slot: 4,
            ..original
        },
        Frame {
            epoch: 0,
            ..original
        },
        Frame {
            elapsed_ms: 86_400_001,
            ..original
        },
    ] {
        assert!(encode(&f, &mut bytes).is_none());
    }
    let mut f = original;
    f.meters[0].rows[0].damage = 1_000_000_001;
    assert!(encode(&f, &mut bytes).is_none());
    let n = encode(&original, &mut bytes).unwrap();
    bytes[28 + 9 + 4] = 255;
    assert!(decode(&bytes[..n]).is_none());
}
#[test]
fn expired_leader_yields_to_active_peer_without_changing_damage() {
    let mut c = Cache::new();
    assert!(c.accept(frame(0, 1), 42, 10_000));
    c.advance(1.0);
    assert!(c.accept(frame(2, 1), 42, 10_000));
    c.advance(0.6);
    assert_eq!(c.leader().unwrap().slot, 2);
    assert_eq!(c.leader().unwrap().meters[0].rows[0].damage, 120);
    let mut f = frame(2, 2);
    f.quest = 43;
    assert!(c.accept(f, 43, 10_000));
    assert!(c.owner(0).is_none());
}
#[test]
fn status_pairs_require_supported_progress_or_zero_unsupported() {
    let mut b = [0; MAX_PACKET];
    for pair in [(101, 100), (1, 0), (-1, 100), (0, -1), (-1, -1), (-2, 100)] {
        let mut f = frame(0, 1);
        f.statuses[0].pairs[0] = pair;
        assert!(encode(&f, &mut b).is_none(), "invalid pair: {pair:?}");
        // Exercise the untrusted decoder as well as callers constructing a Frame.
        let good = frame(0, 1);
        let n = encode(&good, &mut b).unwrap();
        let offset = 28 + 201 + 4;
        b[offset..offset + 2].copy_from_slice(&pair.0.to_le_bytes());
        b[offset + 2..offset + 4].copy_from_slice(&pair.1.to_le_bytes());
        assert!(decode(&b[..n]).is_none(), "invalid wire pair: {pair:?}");
    }
    for pair in [(0, 0), (0, 100), (100, 100), (32767, 32767)] {
        let mut f = frame(0, 1);
        f.statuses[0].pairs[0] = pair;
        let n = encode(&f, &mut b).unwrap();
        assert_eq!(decode(&b[..n]), Some(f));
    }
}
