//! Bounded, versioned HuntLog snapshots. Receipt replaces a snapshot; it never adds damage.
pub const MAX_METERS: usize = 17;
pub const MAX_STATUSES: usize = 10;
pub const MAX_PACKET: usize = 28 + MAX_METERS * (9 + 4 * 48) + MAX_STATUSES * 16;
const MAX_ELAPSED: u32 = 24 * 60 * 60 * 1000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub slot: u8,
    pub color: u8,
    pub weapon: u8,
    pub name: [u8; 32],
    pub name_len: u8,
    pub damage: u64,
    pub hits: u32,
}
impl Row {
    pub const EMPTY: Self = Self {
        slot: 0,
        color: 0,
        weapon: 0,
        name: [0; 32],
        name_len: 0,
        damage: 0,
        hits: 0,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meter {
    pub network: u16,
    pub species: u16,
    pub count: u8,
    pub elapsed_ms: u32,
    pub rows: [Row; 4],
}
impl Meter {
    pub const EMPTY: Self = Self {
        network: 0,
        species: 0,
        count: 0,
        elapsed_ms: 0,
        rows: [Row::EMPTY; 4],
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub network: u16,
    pub species: u16,
    pub pairs: [(i16, i16); 3],
}
impl Status {
    pub const EMPTY: Self = Self {
        network: 0,
        species: 0,
        pairs: [(0, 0); 3],
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub quest: u32,
    pub elapsed_ms: u32,
    pub epoch: u64,
    pub sequence: u32,
    pub slot: u8,
    pub meter_count: u8,
    pub status_count: u8,
    pub meters: [Meter; MAX_METERS],
    pub statuses: [Status; MAX_STATUSES],
}
impl Frame {
    pub const EMPTY: Self = Self {
        quest: 0,
        elapsed_ms: 0,
        epoch: 0,
        sequence: 0,
        slot: 0,
        meter_count: 0,
        status_count: 0,
        meters: [Meter::EMPTY; MAX_METERS],
        statuses: [Status::EMPTY; MAX_STATUSES],
    };
}
fn valid(f: &Frame) -> bool {
    if f.slot >= 4
        || f.meter_count as usize > MAX_METERS
        || f.status_count as usize > MAX_STATUSES
        || f.elapsed_ms > MAX_ELAPSED
        || f.epoch == 0
    {
        return false;
    }
    for (i, m) in f.meters[..f.meter_count as usize].iter().enumerate() {
        if m.count > 4 || m.elapsed_ms > MAX_ELAPSED || (m.network == u16::MAX && m.species != 0) {
            return false;
        }
        if f.meters[..i]
            .iter()
            .any(|p| p.network == m.network && p.species == m.species)
        {
            return false;
        }
        let mut slots = 0u8;
        for r in &m.rows[..m.count as usize] {
            if (r.slot >= 4 && r.slot != 255)
                || r.name_len > 32
                || r.damage > 1_000_000_000
                || r.hits > 10_000_000
                || (r.weapon > 15 && r.weapon != 255)
                || (r.color > 15 && r.color != 255)
            {
                return false;
            }
            if r.slot < 4 {
                let bit = 1 << r.slot;
                if slots & bit != 0 {
                    return false;
                }
                slots |= bit;
            }
            if core::str::from_utf8(&r.name[..r.name_len as usize]).is_err() {
                return false;
            }
        }
    }
    for (i, s) in f.statuses[..f.status_count as usize].iter().enumerate() {
        if s.network == u16::MAX
            || f.statuses[..i]
                .iter()
                .any(|p| p.network == s.network && p.species == s.species)
            || s.pairs
                .iter()
                .any(|&(v, t)| !((v == 0 && t == 0) || (v >= 0 && t > 0 && v <= t)))
        {
            return false;
        }
    }
    true
}
struct Writer<'a> {
    b: &'a mut [u8],
    p: usize,
}
impl Writer<'_> {
    fn put(&mut self, b: &[u8]) {
        self.b[self.p..self.p + b.len()].copy_from_slice(b);
        self.p += b.len();
    }
    fn u8(&mut self, v: u8) {
        self.put(&[v]);
    }
    fn u16(&mut self, v: u16) {
        self.put(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.put(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.put(&v.to_le_bytes());
    }
}
/// Encode only the populated meter/status ranges. No alignment or host endianness leaks.
pub fn encode(f: &Frame, out: &mut [u8]) -> Option<usize> {
    if !valid(f) {
        return None;
    }
    let size = 28 + f.meter_count as usize * 201 + f.status_count as usize * 16;
    if out.len() < size {
        return None;
    }
    let mut w = Writer { b: out, p: 0 };
    w.put(b"HLOG");
    w.u8(1);
    w.u32(f.quest);
    w.u32(f.elapsed_ms);
    w.u64(f.epoch);
    w.u32(f.sequence);
    w.u8(f.slot);
    w.u8(f.meter_count);
    w.u8(f.status_count);
    for m in &f.meters[..f.meter_count as usize] {
        w.u16(m.network);
        w.u16(m.species);
        w.u8(m.count);
        w.u32(m.elapsed_ms);
        for r in &m.rows {
            w.u8(r.slot);
            w.u8(r.color);
            w.u8(r.weapon);
            w.u8(r.name_len);
            w.put(&r.name);
            w.u64(r.damage);
            w.u32(r.hits);
        }
    }
    for s in &f.statuses[..f.status_count as usize] {
        w.u16(s.network);
        w.u16(s.species);
        for &(v, t) in &s.pairs {
            w.u16(v as u16);
            w.u16(t as u16);
        }
    }
    Some(w.p)
}
struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}
impl Reader<'_> {
    fn get<const N: usize>(&mut self) -> Option<[u8; N]> {
        let end = self.p.checked_add(N)?;
        let a = self.b.get(self.p..end)?.try_into().ok()?;
        self.p = end;
        Some(a)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.get::<1>()?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.get()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.get()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.get()?))
    }
}
pub fn decode(bytes: &[u8]) -> Option<Frame> {
    if bytes.len() > MAX_PACKET {
        return None;
    }
    let mut r = Reader { b: bytes, p: 0 };
    if &r.get::<4>()? != b"HLOG" || r.u8()? != 1 {
        return None;
    }
    let mut f = Frame::EMPTY;
    f.quest = r.u32()?;
    f.elapsed_ms = r.u32()?;
    f.epoch = r.u64()?;
    f.sequence = r.u32()?;
    f.slot = r.u8()?;
    f.meter_count = r.u8()?;
    f.status_count = r.u8()?;
    if f.meter_count as usize > MAX_METERS || f.status_count as usize > MAX_STATUSES {
        return None;
    }
    for m in &mut f.meters[..f.meter_count as usize] {
        m.network = r.u16()?;
        m.species = r.u16()?;
        m.count = r.u8()?;
        m.elapsed_ms = r.u32()?;
        for row in &mut m.rows {
            row.slot = r.u8()?;
            row.color = r.u8()?;
            row.weapon = r.u8()?;
            row.name_len = r.u8()?;
            row.name = r.get()?;
            row.damage = r.u64()?;
            row.hits = r.u32()?;
        }
    }
    for s in &mut f.statuses[..f.status_count as usize] {
        s.network = r.u16()?;
        s.species = r.u16()?;
        for p in &mut s.pairs {
            *p = (r.u16()? as i16, r.u16()? as i16);
        }
    }
    if r.p != bytes.len() || !valid(&f) {
        None
    } else {
        Some(f)
    }
}
/// Sequence history survives expiry so delayed duplicates cannot revive stale owners.
pub struct Cache {
    frames: [Frame; 4],
    known: [bool; 4],
    ages: [f32; 4],
    retired: [u64; 4],
    quest: Option<u32>,
    elapsed_ms: u32,
}
impl Cache {
    pub const fn new() -> Self {
        Self {
            frames: [Frame::EMPTY; 4],
            known: [false; 4],
            ages: [0.0; 4],
            retired: [0; 4],
            quest: None,
            elapsed_ms: 0,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    pub fn advance(&mut self, seconds: f32) {
        let delta = if seconds.is_finite() && seconds >= 0.0 {
            seconds
        } else {
            1.5
        };
        for age in &mut self.ages {
            *age = (*age + delta).min(10.0);
        }
    }
    pub fn accept(&mut self, f: Frame, quest: u32, elapsed_ms: u32) -> bool {
        if self.quest != Some(quest) || elapsed_ms.saturating_add(1000) < self.elapsed_ms {
            self.reset();
        }
        self.quest = Some(quest);
        self.elapsed_ms = elapsed_ms;
        if !valid(&f) || f.quest != quest || f.elapsed_ms.abs_diff(elapsed_ms) > 5000 {
            return false;
        }
        let i = f.slot as usize;
        if self.known[i] {
            let old = &self.frames[i];
            if old.epoch == f.epoch {
                if f.sequence.wrapping_sub(old.sequence) as i32 <= 0 {
                    return false;
                }
            } else {
                if self.retired[i] == f.epoch {
                    return false;
                }
                self.retired[i] = old.epoch;
            }
        }
        self.frames[i] = f;
        self.known[i] = true;
        self.ages[i] = 0.0;
        true
    }
    pub fn owner(&self, slot: u8) -> Option<&Frame> {
        let i = slot as usize;
        if i < 4 && self.known[i] && self.ages[i] < 1.5 {
            Some(&self.frames[i])
        } else {
            None
        }
    }
    pub fn leader(&self) -> Option<&Frame> {
        (0..4).find_map(|i| self.owner(i))
    }
}
