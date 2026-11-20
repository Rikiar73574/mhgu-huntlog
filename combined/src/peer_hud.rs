//! Peer snapshots only affect the HUD. Combat, hit accounting and journals stay local.
use crate::{
    DPS, DpsState, MONSTER_DPS, OwnerDps, PlayerName, data::Shared, monsters, peer,
    peer_transport::Transport, quest::Observation, target::Memory,
};
#[derive(Clone, Copy)]
struct Identity {
    object: u32,
    network: u16,
    species: u16,
}
impl Identity {
    const EMPTY: Self = Self {
        object: 0,
        network: 0,
        species: 0,
    };
}
struct Runtime {
    transport: Transport,
    cache: peer::Cache,
    run: u64,
    quest: u32,
    slot: u8,
    peers: u8,
    send_in: f32,
    sequence: u32,
    identities: [Identity; 16],
    leader: Option<(u8, u64)>,
    stamp: Option<(u8, u64, u32)>,
    keys: [(u16, u16); peer::MAX_METERS],
    meters: [DpsState; peer::MAX_METERS],
    count: usize,
}
impl Runtime {
    const fn new() -> Self {
        Self {
            transport: Transport::new(),
            cache: peer::Cache::new(),
            run: 0,
            quest: 0,
            slot: 0,
            peers: 0,
            send_in: 0.0,
            sequence: 0,
            identities: [Identity::EMPTY; 16],
            leader: None,
            stamp: None,
            keys: [(0, 0); peer::MAX_METERS],
            meters: [DpsState::new(); peer::MAX_METERS],
            count: 0,
        }
    }
    fn clear(&mut self) {
        self.transport.close();
        self.cache.reset();
        self.run = 0;
        self.quest = 0;
        self.send_in = 0.0;
        self.identities.fill(Identity::EMPTY);
        self.leader = None;
        self.stamp = None;
        self.count = 0;
        self.peers = 0;
    }
    fn synchronized(&self) -> bool {
        (0..4)
            .filter(|&i| self.peers & (1 << i) != 0 && self.cache.owner(i).is_some())
            .count()
            > 1
    }
    fn remember(&mut self, monsters: &monsters::Snapshot) {
        for m in &monsters.entries[..monsters.count] {
            if let Some(i) = self
                .identities
                .iter()
                .position(|id| id.object == m.object)
                .or_else(|| self.identities.iter().position(|id| id.object == 0))
            {
                self.identities[i] = Identity {
                    object: m.object,
                    network: m.network,
                    species: m.species,
                };
            }
        }
    }
    fn publish(&mut self, m: &impl Memory, quest_ms: u32, monsters: &monsters::Snapshot) {
        self.sequence = self.sequence.wrapping_add(1);
        let mut frame = peer::Frame {
            quest: self.quest,
            elapsed_ms: quest_ms,
            epoch: self.run,
            sequence: self.sequence,
            slot: self.slot,
            ..peer::Frame::EMPTY
        };
        frame.meters[0] = DPS.with(|dps| export(u16::MAX, 0, dps));
        frame.meter_count = 1;
        MONSTER_DPS.with(|pool| {
            for (i, &object) in pool.keys.iter().enumerate() {
                if object == 0 {
                    continue;
                }
                if let Some(id) = self.identities.iter().find(|id| id.object == object) {
                    let at = frame.meter_count as usize;
                    frame.meters[at] = export(id.network, id.species, &pool.states[i]);
                    frame.meter_count += 1;
                }
            }
        });
        for monster in &monsters.entries[..monsters.count] {
            if monster.hp <= 0
                || m.u8(monster.object + 0x7420) != Some(1)
                || m.u8(monster.object + 0x7424) != Some(self.slot)
            {
                continue;
            }
            let at = frame.status_count as usize;
            frame.statuses[at] = peer::Status {
                network: monster.network,
                species: monster.species,
                pairs: core::array::from_fn(|i| monster.ailments[i + 3]),
            };
            frame.status_count += 1;
        }
        // The reference displays its own published snapshot too. Neither peer keeps
        // an independently advancing timer between these shared snapshots.
        self.cache.accept(frame, self.quest, quest_ms);
        self.transport.send(&frame);
    }
    fn refresh_meters(&mut self) {
        if !self.synchronized() {
            return;
        }
        let Some(frame) = (0..4)
            .filter(|&slot| self.peers & (1 << slot) != 0)
            .find_map(|slot| self.cache.owner(slot))
            .copied()
        else {
            return;
        };
        let stamp = (frame.slot, frame.epoch, frame.sequence);
        if self.stamp == Some(stamp) {
            return;
        }
        let leader = (frame.slot, frame.epoch);
        if self.leader != Some(leader) {
            self.count = 0;
            self.leader = Some(leader);
        }
        for meter in &frame.meters[..frame.meter_count as usize] {
            let key = (meter.network, meter.species);
            let index = if let Some(i) = self.keys[..self.count].iter().position(|&id| id == key) {
                i
            } else {
                if self.count == peer::MAX_METERS {
                    continue;
                }
                let i = self.count;
                self.count += 1;
                self.keys[i] = key;
                self.meters[i].reset();
                i
            };
            import(&mut self.meters[index], meter, self.slot);
        }
        self.stamp = Some(stamp);
    }
}
static RUNTIME: Shared<Runtime> = Shared::new(Runtime::new());
fn pointer(p: u32) -> bool {
    (0x08000000..0xa8000000).contains(&p) && p & 3 == 0
}
fn party(m: &impl Memory, base: u32) -> Option<(u8, u8)> {
    let gc = m.u32(base.checked_add(0x188ca48)?)?.checked_add(0x30)?;
    let slot = m.u32(gc)?;
    if slot >= 4 {
        return None;
    }
    let players = m.u32(base.checked_add(0x1885b58)?)?;
    if !pointer(players) {
        return None;
    }
    let mut mask = 0u8;
    for i in 0..4 {
        if m.u32(players + 0x1c + i * 4).is_some_and(pointer) {
            mask |= 1 << i;
        }
    }
    (mask.count_ones() > 1 && mask & (1 << slot) != 0).then_some((slot as u8, mask))
}
pub fn update(
    m: &impl Memory,
    base: u32,
    observation: Observation,
    run: u64,
    seconds: f32,
    monsters: &mut monsters::Snapshot,
) {
    RUNTIME.with(|s| {
        s.cache.advance(seconds);
        let Observation::Quest { id, elapsed, .. } = observation else {
            if observation == Observation::Town {
                s.clear();
            }
            hide_missing(m, &s.cache, s.peers, monsters);
            return;
        };
        let Some((slot, peers)) = party(m, base) else {
            s.clear();
            hide_missing(m, &s.cache, s.peers, monsters);
            return;
        };
        if s.run != run.max(1) || s.quest != id || s.slot != slot {
            s.clear();
            s.run = run.max(1);
            s.quest = id;
            s.slot = slot;
        }
        s.peers = peers;
        s.remember(monsters);
        s.transport.tick(seconds);
        let quest_ms = (elapsed.max(0.0) * 1000.0) as u32;
        for _ in 0..8 {
            if let Some(frame) = s.transport.receive() {
                if frame.slot != slot && peers & (1 << frame.slot) != 0 {
                    s.cache.accept(frame, id, quest_ms);
                }
            }
        }
        s.send_in -= seconds;
        if s.send_in <= 0.0 {
            s.send_in = 0.25;
            s.publish(m, quest_ms, monsters);
        }
        s.refresh_meters();
        hide_missing(m, &s.cache, s.peers, monsters);
    });
}
// Only the monster's current AI owner can supply buildup. Expired or absent
// reports remove the three rows instead of presenting stale replica zeros.
fn hide_missing(
    m: &impl Memory,
    cache: &peer::Cache,
    peers: u8,
    monsters: &mut monsters::Snapshot,
) {
    for monster in &mut monsters.entries[..monsters.count] {
        if m.u8(monster.object + 0x7420) == Some(1) {
            continue;
        }
        let report = m
            .u8(monster.object + 0x7424)
            .filter(|&slot| slot < 4 && peers & (1 << slot) != 0)
            .and_then(|slot| cache.owner(slot))
            .and_then(|frame| {
                frame.statuses[..frame.status_count as usize]
                    .iter()
                    .find(|s| s.network == monster.network && s.species == monster.species)
            });
        for i in 0..3 {
            monster.ailments[i + 3] = report.map_or((0, 0), |s| s.pairs[i]);
        }
    }
}
pub fn following() -> bool {
    RUNTIME.with(|s| {
        s.synchronized()
            && (0..4)
                .filter(|&slot| s.peers & (1 << slot) != 0)
                .find_map(|slot| s.cache.owner(slot))
                .is_some_and(|f| f.slot != s.slot)
    })
}
pub fn meter(network: u16, species: u16) -> Option<DpsState> {
    RUNTIME.with(|s| {
        if !s.synchronized() {
            return None;
        }
        s.keys[..s.count]
            .iter()
            .position(|&key| key == (network, species))
            .map(|i| s.meters[i])
    })
}
fn export(network: u16, species: u16, dps: &DpsState) -> peer::Meter {
    let mut out = peer::Meter {
        network,
        species,
        count: dps.count as u8,
        elapsed_ms: (dps.elapsed.max(0.0) * 1000.0) as u32,
        ..peer::Meter::EMPTY
    };
    for (to, from) in out.rows.iter_mut().zip(&dps.owners[..dps.count]) {
        *to = peer::Row {
            slot: from.slot,
            color: from.color,
            weapon: from.weapon,
            name: from.name.bytes,
            name_len: from.name.len,
            damage: from.damage,
            hits: from.hits,
        };
    }
    out
}
fn import(dps: &mut DpsState, meter: &peer::Meter, slot: u8) {
    let elapsed = meter.elapsed_ms as f32 / 1000.0;
    if elapsed < dps.elapsed {
        dps.reset();
    }
    dps.count = meter.count as usize;
    dps.total = 0;
    dps.elapsed = elapsed;
    dps.active = meter.count > 0;
    dps.in_quest = true;
    for (to, from) in dps.owners.iter_mut().zip(&meter.rows) {
        *to = OwnerDps {
            owner: 0,
            local: from.slot == slot,
            damage: from.damage,
            hits: from.hits,
            slot: from.slot,
            color: from.color,
            weapon: from.weapon,
            name: PlayerName {
                bytes: from.name,
                len: from.name_len,
            },
        };
    }
    for row in &dps.owners[..dps.count] {
        dps.total = dps.total.saturating_add(row.damage);
    }
    dps.sample_history();
}

#[cfg(test)]
#[path = "peer_hud_cases.rs"]
mod tests;
