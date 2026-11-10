//! MHGU 1.4.0 本地伤害数字：采集命中 → 伤害入账 → 游戏投影 → Forge 绘制。
#![no_std]

mod data;
mod monsters;
mod weakness;
mod quest;
mod recorder;
mod monster_size;
mod hunt_runtime;
mod status_events;mod charge;
mod item_events;mod part_names;mod parts;mod player_details;mod rewards;
mod target;
mod layout;
mod timers;
mod map_timers;
mod debuffs;
mod hook_policy;
static DEBUFF_ICONS: Shared<[debuffs::Icon;16]> = Shared::new([debuffs::Icon::EMPTY;16]);
static MAP_MARKERS: Shared<[map_timers::Marker;32]> = Shared::new([map_timers::Marker::EMPTY;32]);
static MAP_FRAME: AtomicU32 = AtomicU32::new(0);
static TIMERS: Shared<timers::Snapshot> = Shared::new(timers::Snapshot::EMPTY);
static TIMERS_AVAILABLE: AtomicBool = AtomicBool::new(false);
static MONSTERS: data::Shared<monsters::Snapshot> = data::Shared::new(monsters::Snapshot::EMPTY);
mod icons;
#[path = "../shared/plugin_config.rs"]
mod plugin_config;

use core::{
    arch::asm,
    ffi::c_void,
    fmt::{self, Write},
    mem::{self, MaybeUninit},
    ptr,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
use data::*;
use forge::{
    imgui,
    log::prelude::{error, info, warn},
    sys::imgui::{ImDrawFlags, ImFont_GetFontBaked, ImGuiInputTextFlags, ImVec2},
};

// Forge 入口、更新和绘图。
// 字体仅在 ImGui 初始化时写入，随后由同一渲染线程读取。
static mut CUSTOM_FONT: Option<imgui::ImFont> = None;
static CONFIG_LOADED: AtomicBool = AtomicBool::new(false);
static IMGUI_READY: AtomicBool = AtomicBool::new(false);
static MENU: Shared<plugin_config::Edit<Config>> =
    Shared::new(plugin_config::Edit::new(Config::DEFAULT));
static FONT_INPUT: Shared<[u8; 168]> = Shared::new([0; 168]);

// Debug build: record every valid contact/damage event, including teammates,
// into the private huntlog:/dps_debug.log mount. The display path still only shows local hits.
const DEBUG_LOG_CAP: usize = 262144;

// true for the log-collection build; false for the clean DPS panel build.
const DEBUG_LOG_ENABLED: bool = false;

// false: keep hooks/DPS but do not draw this plugin's own damage numbers.
const SHOW_LOCAL_NUMBERS: bool = false;

struct DebugLog {
    bytes: [u8; DEBUG_LOG_CAP],
    len: usize,
    truncated: bool,
    elapsed: f32,
    session_local: u32,
    session_room: u8,
    seen: [u32; 8],
    seen_len: usize,
    hits: u32,
}

impl DebugLog {
    const fn new() -> Self {
        Self {
            bytes: [0; DEBUG_LOG_CAP],
            len: 0,
            truncated: false,
            elapsed: 0.0,
            session_local: 0,
            session_room: 0,
            seen: [0; 8],
            seen_len: 0,
            hits: 0,
        }
    }

    fn push(&mut self, args: fmt::Arguments<'_>) {
        if self.truncated {
            return;
        }
        if self.write_fmt(args).is_err() {
            self.truncated = true;
        }
    }

    fn flush(&mut self) {
        if !DEBUG_LOG_ENABLED || self.len == 0 {
            return;
        }
        let text = unsafe { core::str::from_utf8_unchecked(&self.bytes[..self.len]) };
        if let Err(error) = plugin_config::DEBUG.save(text) {
            warn!("dps debug log: {error}");
        }
        self.elapsed = 0.0;
    }

    fn tick(&mut self, _dt: f32) {
        if !DEBUG_LOG_ENABLED {
            return;
        }
        // Never write periodically: save-data mount/commit stalls the game.
        // Flush only when the buffer is nearly full, or on the explicit triggers.
        if self.len >= DEBUG_LOG_CAP - 1024 {
            self.flush();
        }
    }

    fn hex(&mut self, label: &str, pointer: u32, length: usize) {
        if pointer < 0x10000 {
            return;
        }
        self.push(format_args!("[{}] ptr={:08X} data=", label, pointer));
        let words = length / 4;
        for i in 0..words {
            let value = unsafe { ptr::read_volatile((pointer + i as u32 * 4) as *const u32) };
            if self.write_fmt(format_args!("{:08X}", value)).is_err() {
                self.truncated = true;
                break;
            }
            if i + 1 < words {
                let _ = self.write_str(" ");
            }
        }
        let _ = self.write_str("
");
    }

    fn session_snapshot(&mut self, base: u32) {
        if !DEBUG_LOG_ENABLED {
            return;
        }
        let local = unsafe { local_player(base) };
        if local < 0x10000 {
            return;
        }
        let room = unsafe { read_u8(local + 0x1054) };
        if self.session_local == local && self.session_room == room {
            return;
        }
        self.session_local = local;
        self.session_room = room;
        let manager = unsafe { read_u32(base + 0x01885B58) };
        self.push(format_args!(
            "[SESSION] base={:08X} manager={:08X} local={:08X} room={:02X}
",
            base, manager, local, room
        ));
        self.hex("MANAGER", manager, 0x400);
        self.hex("LOCAL", local, 0x400);
    }

    fn owner_snapshot(&mut self, owner: u32, local: u32) {
        if !DEBUG_LOG_ENABLED || owner < 0x10000 {
            return;
        }
        if self.seen[..self.seen_len].contains(&owner) {
            return;
        }
        if self.seen_len < self.seen.len() {
            self.seen[self.seen_len] = owner;
            self.seen_len += 1;
        }
        let room = unsafe { read_u8(owner + 0x1054) };
        let is_local = if owner == local { 1 } else { 0 };
        self.push(format_args!(
            "[OWNER] ptr={:08X} local={} room={:02X}
",
            owner, is_local, room
        ));
        self.hex("OWNER", owner, 0x400);
    }

    fn hit(&mut self, contact: &Contact, damage: i32, raw: i32, mode: i32) {
        if !DEBUG_LOG_ENABLED {
            return;
        }
        self.hits += 1;
        let number = self.hits;
        let frame = FRAME.load(Ordering::Relaxed);
        let owner = contact.owner;
        let local = if contact.local { 1 } else { 0 };
        let room = contact.epoch.room;
        let attack = contact.attack;
        let monster = contact.monster;
        let slot = contact.slot;
        let part_index = contact.part_index;
        let part = contact.part;
        let world = contact.world;
        self.push(format_args!(
            "[HIT] n={} f={} owner={:08X} local={} pslot={} room={:02X} attack={:08X} monster={:08X} slot={} part={} partptr={:08X} dmg={} raw={} mode={} world={:08X},{:08X},{:08X}
",
            number, frame, owner, local, contact.player_slot, room, attack, monster, slot, part_index, part,
            damage, raw, mode,
            world[0].to_bits(), world[1].to_bits(), world[2].to_bits()
        ));
    }
}

impl fmt::Write for DebugLog {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.len + text.len();
        if end > self.bytes.len() {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

static DEBUG_LOG: Shared<DebugLog> = Shared::new(DebugLog::new());

// 游戏内名字是 UTF-8；manager 的玩家数据块最多保留 32 字节。
#[derive(Clone, Copy)]
struct PlayerName {
    bytes: [u8; 32],
    len: u8,
}

impl PlayerName {
    const EMPTY: Self = Self {
        bytes: [0; 32],
        len: 0,
    };

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// Minimal per-owner DPS aggregation.
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
        slot: 0xFF,
        color: 0xFF,
        weapon: 0xFF,
        name: PlayerName::EMPTY,
    };
}

// 整场曲线固定保留 600 个点；满了就合并相邻点，越长的任务时间分辨率越低，
// 但每帧顶点数、状态大小和绘制成本保持不变。
const HISTORY_CAP: usize = 600;
const SAMPLE_INTERVAL: f32 = 1.0;

#[derive(Clone, Copy)]
struct HistorySample {
    time: f32,
    damage: [u32; 4],
}

impl HistorySample {
    const EMPTY: Self = Self {
        time: 0.0,
        damage: [0; 4],
    };
}

#[derive(Clone, Copy)]
struct DpsState {
    owners: [OwnerDps; 4],
    count: usize,
    total: u64,
    elapsed: f32,
    active: bool,
    visible: bool,
    in_quest: bool,
    result_frames: u32,
    history: [HistorySample; HISTORY_CAP],
    history_len: usize,
    last_sample: f32,
}

impl DpsState {
    const fn new() -> Self {
        Self {
            owners: [OwnerDps::EMPTY; 4],
            count: 0,
            total: 0,
            elapsed: 0.0,
            active: false,
            visible: true,
            in_quest: false,
            result_frames: 0,
            history: [HistorySample::EMPTY; HISTORY_CAP],
            history_len: 0,
            last_sample: 0.0,
        }
    }

    fn reset(&mut self) {
        self.owners = [OwnerDps::EMPTY; 4];
        self.count = 0;
        self.total = 0;
        self.elapsed = 0.0;
        self.active = false;
        self.history = [HistorySample::EMPTY; HISTORY_CAP];
        self.history_len = 0;
        self.last_sample = 0.0;
    }

    fn reset_now(&mut self) {
        self.reset();
    }


    // Save/result screens have no local player object. Require a short
    // debounce so area loading cannot clear an active run.
    fn on_result_screen(&mut self, result: bool) {
        if result {
            self.result_frames = self.result_frames.saturating_add(1);
            if self.result_frames >= 60 {
                self.reset();
                self.in_quest = false;
            }
        } else {
            self.result_frames = 0;
            if !self.in_quest {
                self.in_quest = true;
            }
        }
    }

    fn add(&mut self, owner: u32, local: bool, damage: i32, player_slot: u8) {
        if damage <= 0 {
            return;
        }
        if !self.in_quest {
            self.reset();
            self.in_quest = true;
        }
        let index = if player_slot < 4 {
            self.owners[..self.count]
                .iter()
                .position(|row| row.slot == player_slot)
        } else {
            self.owners[..self.count]
                .iter()
                .position(|row| row.slot == 0xFF && row.owner == owner)
        };
        let index = match index {
            Some(index) => index,
            None => {
                if self.count >= self.owners.len() {
                    return;
                }
                self.owners[self.count] = OwnerDps {
                    owner,
                    local,
                    damage: 0,
                    hits: 0,
                    slot: player_slot,
                    color: owner_color(owner, player_slot),
                    weapon: owner_weapon(owner),
                    name: unsafe { owner_name(GAME_BASE.load(Ordering::Relaxed), owner) },
                };
                self.count += 1;
                self.count - 1
            }
        };
        let row = &mut self.owners[index];
        row.damage = row.damage.saturating_add(damage as u64);
        row.hits = row.hits.saturating_add(1);
        if local {
            row.local = true;
        }
        if row.slot == 0xFF {
            row.slot = player_slot;
        }
        let color = owner_color(owner, player_slot);
        if color < 4 {
            row.color = color;
        }
        let weapon = owner_weapon(owner);
        if weapon != 0xFF {
            row.weapon = weapon;
        }
        let name = unsafe { owner_name(GAME_BASE.load(Ordering::Relaxed), owner) };
        if !name.is_empty() {
            row.name = name;
        }
        self.total = self.total.saturating_add(damage as u64);
        self.active = true;
    }

    fn decimate_history(&mut self) {
        // 保留第一点和每个偶数点，再补上最后一点；累计伤害值不会被平均，
        // 曲线末端始终对应真实的当前总伤害。
        let mut source = 0usize;
        let mut target = 0usize;
        while source < self.history_len {
            self.history[target] = self.history[source];
            target += 1;
            source += 2;
        }
        let last = self.history_len - 1;
        if target == 0 || self.history[target - 1].time != self.history[last].time {
            self.history[target] = self.history[last];
            target += 1;
        }
        self.history_len = target;
    }

    fn tick(&mut self, dt: f32) {
        if !self.active || !self.in_quest {
            return;
        }
        self.elapsed += if dt.is_finite() && dt > 0.0 { dt } else { 1.0 / 60.0 };
        if self.history_len == 0 || self.elapsed - self.last_sample >= SAMPLE_INTERVAL {
            let mut damage = [0u32; 4];
            for i in 0..self.count {
                let row = self.owners[i];
                let index = row_color(&row, i);
                damage[index.min(3)] = row.damage.min(u32::MAX as u64) as u32;
            }
            let sample = HistorySample {
                time: self.elapsed,
                damage,
            };
            if self.history_len == HISTORY_CAP {
                self.decimate_history();
            }
            if self.history_len < HISTORY_CAP {
                self.history[self.history_len] = sample;
                self.history_len += 1;
            }
            self.last_sample = self.elapsed;
        }
    }

}

fn owner_color(owner: u32, fallback: u8) -> u8 {
    if owner < 0x10000 {
        return if fallback < 4 { fallback } else { 0xFF };
    }
    let value = unsafe { read_u32(owner + 0x100) } & 0xF00;
    match value {
        0x100 => 0,
        0x200 => 1,
        0x400 => 3,
        0x800 => 2,
        _ => {
            if fallback < 4 {
                fallback
            } else {
                0xFF
            }
        }
    }
}

fn owner_weapon(owner: u32) -> u8 {
    if owner < 0x10000 {
        return 0xFF;
    }
    let pointer = unsafe { read_u32(owner + 0x13C4) };
    // Switch 的用户堆也可能落在 0x8xxxxxxx；只排除低地址和接近 4GB 的非法范围。
    if pointer < 0x10000 || pointer > 0xFFFF_0000 - 0x14C {
        return 0xFF;
    }
    let value = unsafe { read_u8(pointer + 0x14C) };
    if value <= 0x0F {
        value
    } else {
        0xFF
    }
}

unsafe fn owner_name(base: u32, owner: u32) -> PlayerName {
    if owner < 0x10000 || base < 0x10000 {
        return PlayerName::EMPTY;
    }
    let manager = unsafe { read_u32(base + 0x01885B58) };
    if manager < 0x10000 {
        return PlayerName::EMPTY;
    }
    let local = unsafe { local_player(base) };
    let mut local_fallback = PlayerName::EMPTY;
    for i in 0..5u32 {
        let block = manager + 0x300 + i * 0x560;
        let name_ptr = block + 0x290;
        let mut name = PlayerName::EMPTY;
        for j in 0..32u32 {
            let byte = unsafe { read_u8(name_ptr + j) };
            if byte == 0 {
                break;
            }
            name.bytes[name.len as usize] = byte;
            name.len += 1;
        }
        if name.is_empty() || core::str::from_utf8(&name.bytes[..name.len as usize]).is_err() {
            continue;
        }
        let owner_ptr = unsafe { read_u32(block + 0x288) };
        if owner_ptr >= 0x10000 {
            if owner_ptr == owner {
                return name;
            }
        } else if owner == local && local_fallback.is_empty() {
            // 本地猎人名块（普通任务通常在这里）；若另有猫名块指向本地玩家，
            // 上面的精确匹配会优先返回。
            local_fallback = name;
        }
    }
    local_fallback
}

fn row_icon(row: &OwnerDps) -> usize {
    if row.weapon == 15 {
        PALICO_ICON
    } else if row.weapon < 15 {
        row.weapon as usize
    } else {
        0xFF
    }
}

fn row_label(row: &OwnerDps, show_names: bool) -> &str {
    let name = row.name.as_str();
    if show_names && !name.is_empty() {
        return name;
    }
    if row.local {
        "You"
    } else {
        match row.slot {
            0 => "P1",
            1 => "P2",
            2 => "P3",
            3 => "P4",
            _ => "P?",
        }
    }
}

fn row_color(row: &OwnerDps, index: usize) -> usize {
    if row.color < 4 {
        row.color as usize
    } else if row.slot < 4 {
        row.slot as usize
    } else {
        index
    }
}

static DPS: Shared<DpsState> = Shared::new(DpsState::new());
struct MonsterDps {run:u64,keys:[u32;16],states:[DpsState;16]}
static MONSTER_DPS:Shared<MonsterDps>=Shared::new(MonsterDps{run:0,keys:[0;16],states:[DpsState::new();16]});


// 猎猫图标槽比武器图标低 2px；按 20x20 图标槽整体上移对齐。
const PALICO_ICON: usize = 15;
const PALICO_ICON_LIFT: f32 = 2.0;
static FRAME: AtomicU32 = AtomicU32::new(0);
static HOOK_ATTEMPTED: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn forge_onLoad(params: *mut forge::sys::init::PluginInitParams) {
    unsafe {
        (*params).required_version = forge::REQUIRED_VERSION;
        // The C PluginInitParam has ForgeVersion (6 bytes) followed by name[64].
        let base = params as *mut u8;
        let name = b"HuntLog";
        core::ptr::copy_nonoverlapping(name.as_ptr(), base.add(6), name.len());
        *base.add(6 + name.len()) = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn forge_onInit() {
    forge::log::init().expect("Failed to initialize logger");
    main();
}

fn main() {
    let base = forge::mem::text_addr();
    GAME_BASE.store(base, Ordering::Relaxed);
    let timers_ok=timers::compatible(&monsters::GameMemory{base},base);
    TIMERS_AVAILABLE.store(timers_ok,Ordering::Relaxed);
    if !timers_ok {warn!("HuntLog: timer signatures mismatch; timer widget disabled");}
    try_load_config();
    // Install after all plugins' forge_onInit so Damage Numbers can install first.
    info!(
        "mhgu-dps-meter v{}: hooks deferred, main=0x{base:08X}",
        env!("CARGO_PKG_VERSION")
    );
}

fn try_load_config() -> bool {
    if CONFIG_LOADED.load(Ordering::Acquire) { return false; }
    let Some(config) = load_config() else { return false; };
    MENU.with(|edit| *edit = plugin_config::Edit::new(config));
    set_font_input(config);
    STATE.with(|state| state.config = config);
    ENABLED.store(config.enabled, Ordering::Relaxed);
    CONFIG_LOADED.store(true, Ordering::Release);
    info!("HuntLog: save settings applied");
    true
}

fn load_config() -> Option<Config> {
    let mut bytes = [0; CONFIG_MAX_BYTES];
    match plugin_config::DAMAGE.read(&mut bytes) {
        Ok(text) => match Config::parse(text) {
            Ok(config) => return Some(config),
            Err(error) => warn!("damage save INI line {}: {}", error.line, error.message),
        },
        // Account initialization can follow plugin initialization. Retry later
        // instead of retaining defaults for the entire session.
        Err(plugin_config::ReadError::Unavailable) => return None,
        Err(error) => warn!("damage save INI: {error:?}; using defaults"),
    }
    Some(Config::DEFAULT)
}

fn file_exists(path: &core::ffi::CStr) -> bool {
    // 直接传入已有 C 字符串，避免 SDK 的 CString 包装引入 malloc/free 导入。
    unsafe { forge::sys::fs::forge_fs_fileExists(path.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn forge_onImGuiInit(
    _context: *mut c_void,
    _alloc: *mut c_void,
    _free: *mut c_void,
) {
    try_load_config();
    load_custom_font();
    IMGUI_READY.store(true, Ordering::Release);
}

fn load_custom_font() {
    unsafe { CUSTOM_FONT = None };
    let config = STATE.with(|state| state.config);
    if let Some(file) = config.font_file {
        let path = file.path();
        let font = if file_exists(path) {
            imgui::ImFont::load(path)
        } else {
            None
        };
        match font {
            Some(font) => {
                unsafe { CUSTOM_FONT = Some(font) };
                info!(
                    "mhgu-dps-meter: loaded font {}",
                    path.to_str().unwrap()
                );
            }
            None => warn!(
                "mhgu-dps-meter: could not load font {}; using Forge font",
                path.to_str().unwrap()
            ),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn forge_onUpdate(dt: f32) {
    if !CONFIG_LOADED.load(Ordering::Acquire) && plugin_config::storage_ready() && try_load_config()
        && IMGUI_READY.load(Ordering::Acquire) {
        // Update precedes Forge's next ImGui frame; the font atlas is unlocked.
        load_custom_font();
    }
    let monsters = monsters::capture();
    MONSTERS.with(|value| *value = monsters);
    // 在游戏更新结束后投影；NVN 绘制回调只读插件快照，不访问游戏对象。
    let base = GAME_BASE.load(Ordering::Relaxed);
    FRAME.fetch_add(1, Ordering::Relaxed);
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 1.0 / 60.0 };
    if !INSTALLED.load(Ordering::Acquire) && !HOOK_ATTEMPTED.swap(true, Ordering::AcqRel) {
        if compatible(base) && unsafe { install_hooks(base) } {
            INSTALLED.store(true, Ordering::Release);
            info!("mhgu-dps-meter: chained hooks installed after first update");
        } else {
            error!("mhgu-dps-meter: hook installation failed");
        }
    }
    if DEBUG_LOG_ENABLED {
        DEBUG_LOG.with(|log| {
            log.tick(dt);
            log.session_snapshot(base);
        });
    }
    update_timers(base,&monsters);
    hunt_runtime::update(base,quest::frame_seconds(&monsters::GameMemory{base},base,dt),&monsters);
    let run=hunt_runtime::status().1;
    MONSTER_DPS.with(|pool|{if pool.run!=run{pool.run=run;pool.keys=[0;16];for state in &mut pool.states{state.reset();}DPS.with(|s|s.reset());}});
    let display = unsafe { display_area(base) };
    if display.width >= 320.0 && display.height >= 240.0 {
        STATE.with(|state| state.set_display(display));
    }
    let mut snapshot = STATE.with(|state| state.snapshot());
    let player = unsafe { local_player(base) };
    // Quest transitions, not temporary player absence, reset DPS (above).
    // Quest/room transitions flush even when the last damage number has already expired.
    if player >= 0x10000
        && snapshot.epoch.player != 0
        && (player != snapshot.epoch.player || unsafe { epoch(player) } != snapshot.epoch)
    {
        if DEBUG_LOG_ENABLED {
            DEBUG_LOG.with(|log| log.flush());
        }
        STATE.with(|state| state.reset_epoch());
        return;
    }
    if !snapshot.any_active() {
        return;
    }
    if player < 0x10000 {
        return;
    }
    let projection = unsafe { Projection::current(base) };
    snapshot.display = unsafe { display_area(base) };
    for event in snapshot.events.iter_mut().filter(|event| event.active) {
        event.visible = false;
        if let Some(screen) = unsafe { projection.project(event.world) } {
            event.screen = screen;
            event.visible = true;
        }
    }
    STATE.with(|state| state.commit_projection(&snapshot));
}

// A missing Forge drawing handle must skip rendering, never reach native ImGui.
// Report once so log.txt distinguishes a bridge failure from timer memory reads.
fn foreground_draw_list() -> Option<imgui::DrawList> {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    let draw = imgui::DrawList::foreground();
    if draw.raw().is_null() {
        if !REPORTED.swap(true, Ordering::Relaxed) {
            warn!("HuntLog: Forge returned a null foreground draw list; skipping overlay drawing");
        }
        None
    } else {
        Some(draw)
    }
}

#[forge::imgui_free_render]
fn draw_damage() {
    draw_timers();
    let seconds = imgui::get_io().DeltaTime;
    let seconds = if seconds.is_finite() && seconds > 0.0 {
        seconds
    } else {
        1.0 / 60.0
    };
    DPS.with(|dps| dps.tick(seconds));
    MONSTER_DPS.with(|pool|{for state in &mut pool.states{state.tick(seconds);}});
    let snapshot = STATE.with(|state| state.advance(seconds));
    if !SHOW_LOCAL_NUMBERS {
        draw_dps();
        return;
    }
    let config = snapshot.config;
    let Some(draw) = foreground_draw_list() else { return; };
    let font = unsafe { CUSTOM_FONT }.unwrap_or_else(imgui::ImFont::current);
    // Forge 的 DisplaySize 是整张交换纹理；掌机画面只使用其中的 1280×720。
    let scale = snapshot.display.scale();
    imgui::push_font(&font, Some(config.font_size * scale));
    let mut visible = 0;
    for event in snapshot
        .events
        .iter()
        .filter(|event| event.active && event.visible)
    {
        let size = (config.font_size + 4.0 * (1.0 - event.age / 0.15).max(0.0)) * scale;
        let text = event.digits.as_str();
        let text_size = imgui::calc_text_size(text, false, 0.0);
        let width = text_size.x * size / imgui::ImFont::current_size();
        let position = snapshot.display.position(event.screen);
        let x = position[0] - width * 0.5 + ((event.serial % 5) as f32 - 2.0) * 6.0 * scale;
        let y = position[1]
            - size
            - ((event.age / config.full_opacity).min(1.0) * 30.0 + (event.serial % 3) as f32 * 5.0)
                * scale;
        let alpha = event.alpha(&config);
        let outline = imgui::color(
            config.outline_color[0],
            config.outline_color[1],
            config.outline_color[2],
            (u16::from(alpha) * u16::from(config.outline_color[3]) / 255) as u8,
        );
        let edge = config.outline_width * scale;
        if edge > 0.0 {
            for (dx, dy) in [
                (-edge, 0.0),
                (edge, 0.0),
                (0.0, -edge),
                (0.0, edge),
                (-edge, -edge),
                (-edge, edge),
                (edge, -edge),
                (edge, edge),
            ] {
                draw.add_text_with_font(&font, size, ImVec2::new(x + dx, y + dy), outline, text);
            }
        }
        let rgb = if event.yellow {
            config.highlight_color
        } else {
            config.normal_color
        };
        let color = imgui::color(
            rgb[0],
            rgb[1],
            rgb[2],
            (u16::from(alpha) * u16::from(rgb[3]) / 255) as u8,
        );
        draw.add_text_with_font(&font, size, ImVec2::new(x, y), color, text);
        visible += 1;
    }
    imgui::pop_font();
    VISIBLE.store(visible, Ordering::Relaxed);
    draw_dps();
}

fn dps_color(index: usize, alpha: u8) -> u32 {
    const COLORS: [(u8, u8, u8); 4] = [
        (255, 107, 107),
        (120, 185, 255),
        (92, 214, 92),
        (255, 212, 92),
    ];
    let color = COLORS[index.min(3)];
    imgui::color(color.0, color.1, color.2, alpha)
}

fn font_ascent(font: &imgui::ImFont, size: f32) -> f32 {
    unsafe {
        let baked = ImFont_GetFontBaked(font.raw(), size, 1.0);
        if baked.is_null() {
            size * 0.8
        } else {
            (*baked).Ascent
        }
    }
}

fn measure_text(font: &imgui::ImFont, size: f32, text: &str) -> f32 {
    imgui::push_font(font, Some(size));
    let current = imgui::ImFont::current_size();
    let width = imgui::calc_text_size(text, false, 0.0).x;
    imgui::pop_font();
    if current > 0.0 {
        width * size / current
    } else {
        width
    }
}

fn draw_pixel_icon(
    draw: &imgui::DrawList,
    icon: &icons::Icon,
    left: f32,
    bottom: f32,
    target: f32,
    color: u32,
) {
    let logical_w = (icon.width / 2) as usize;
    let logical_h = (icon.height / 2) as usize;
    if logical_w == 0 || logical_h == 0 {
        return;
    }
    // Fit the source to its reserved HUD space, including higher-detail classic icons.
    let cell = target / logical_w as f32;
    let icon_left = ((left).max(0.0) + 0.5) as u32 as f32;
    let bottom_px = ((bottom).max(0.0) + 0.5) as u32 as f32;
    let icon_top = bottom_px - logical_h as f32 * cell;
    let cell_x = cell as f32;
    let cell_y = cell as f32;
    let width = icon.width as usize;

    let tint_r = color & 0xFF;
    let tint_g = (color >> 8) & 0xFF;
    let tint_b = (color >> 16) & 0xFF;
    let tint_a = (color >> 24) & 0xFF;
    let block = |lx: usize, ly: usize| -> (u8, u8, u8, u8) {
        let sx = (lx * 2).min(width - 1);
        let sy = (ly * 2).min(icon.height as usize - 1);
        let px = (sy * width + sx) * 4;
        (
            icon.pixels[px],
            icon.pixels[px + 1],
            icon.pixels[px + 2],
            icon.pixels[px + 3],
        )
    };
    let tinted = |pixel: (u8, u8, u8, u8)| -> u32 {
        let r = pixel.0 as u32 * tint_r / 255;
        let g = pixel.1 as u32 * tint_g / 255;
        let b = pixel.2 as u32 * tint_b / 255;
        let a = pixel.3 as u32 * tint_a / 255;
        r | (g << 8) | (b << 16) | (a << 24)
    };

    for ly in 0..logical_h {
        let mut lx = 0usize;
        while lx < logical_w {
            let pixel = block(lx, ly);
            if pixel.3 == 0 {
                lx += 1;
                continue;
            }
            let start = lx;
            lx += 1;
            while lx < logical_w {
                let next = block(lx, ly);
                if next != pixel {
                    break;
                }
                lx += 1;
            }
            draw.add_rect_filled(
                ImVec2::new(
                    icon_left + start as f32 * cell_x,
                    icon_top + ly as f32 * cell_y,
                ),
                ImVec2::new(
                    icon_left + lx as f32 * cell_x,
                    icon_top + (ly + 1) as f32 * cell_y,
                ),
                tinted(pixel),
                0.0,
            );
        }
    }
}
fn monster_label(m:&monsters::Monster,config:&Config)->&'static str {
    if m.object==0 {"Rathalos (preview)"} else if !m.revealed && !config.show_undiscovered_name {"???"} else {monsters::name(m.species).unwrap_or("???")}
}
fn status_rows(m: &monsters::Monster)->usize {
    if !m.selected || !m.revealed {return 0;}
    (m.ailments.iter().copied().enumerate().filter(|&(i,p)|status_events::buildup_visible(i,p)).count()+usize::from(m.rage>0.0)+3)/4+usize::from(m.charge.is_some())
}
fn detail_rows(m:&monsters::Monster,c:&Config)->usize {
    usize::from(c.show_location) + usize::from(c.show_weakness && m.weakness!=0)
    + m.parts.map_or(0,|snapshot|snapshot.parts.iter().filter(|p|p.visible(c.always_show_parts,c.show_breakable_hp,c.show_all_parts_hp)).map(|p|1+usize::from(p.secondary()&&(c.show_breakable_hp||c.show_all_parts_hp))).sum())
}
fn draw_dps_curve(dps: &DpsState, config: &Config, display: DisplayArea, monsters: &monsters::Snapshot) {
    let scale = (display.height / 1080.0).min(2.0) * config.hud_scale;
    draw_panel_scaled(dps,config,display,monsters,scale,0);
}
fn draw_panel_scaled(dps:&DpsState,config:&Config,display:DisplayArea,monsters:&monsters::Snapshot,scale:f32,attempt:u8) {
    let has_dps = dps.count > 0;
    let monster_row_h = if config.show_hp {35.0} else {22.0} * scale;
    let status_h=17.0*scale;
    let monster_height: f32 = monsters.entries[..monsters.count].iter().map(|m|
        monster_row_h + (status_rows(m) + detail_rows(m,config)) as f32 * status_h
    ).sum();
    let font = unsafe { CUSTOM_FONT }.unwrap_or_else(imgui::ImFont::current);
    let font_size = 13.0 * scale;
    let small_font = font_size * 0.72;
    let name_font = font_size * 1.15;
    let icon_size = name_font * 0.95;
    let header_h = font_size + 4.0 * scale;
    let row_text_offset = 5.0 * scale;
    let inline_stats = config.dps_inline_stats && !config.dps_show_hits;
    let two_line = !inline_stats && config.dps_show_damage && config.dps_show_dps;
    let name_bottom = row_text_offset + name_font;
    let two_line_bottom = font_size * 0.92 - 2.0 * scale + small_font;
    let content_bottom = if two_line { two_line_bottom } else { name_bottom };
    let row_h = if config.dps_show_percent_bar {
        if two_line {
            name_font + small_font + 8.0 * scale
        } else {
            name_bottom + 8.0 * scale
        }
    } else {
        content_bottom + 4.0 * scale
    };
    let pad = 7.0 * scale;
    let show_chart = has_dps && config.dps_show_curve;
    let chart_height = if show_chart {
        config.dps_curve_height * scale
    } else {
        0.0
    };
    // 曲线下方与第一行的留白，等于普通玩家行之间由进度条底部到下一行
    // 顶部的空白；不显示曲线时不需要额外留白。
    let first_row_gap = if show_chart {
        name_font - font_size + 2.0 * scale
    } else {
        0.0
    };

    let mut rows = dps.owners;
    for i in 0..dps.count {
        for j in (i + 1)..dps.count {
            if rows[j].damage > rows[i].damage {
                rows.swap(i, j);
            }
        }
    }

    let icon_space = if config.dps_show_weapon {
        icon_size + 6.0 * scale
    } else {
        0.0
    };
    let stat_gap = 8.0 * scale;
    let mut labels: [plugin_config::Text<64>; 4] =
        core::array::from_fn(|_| plugin_config::Text::new());
    let mut name_widths = [0.0f32; 4];
    let mut stats_widths = [0.0f32; 4];
    let mut max_percent_w = 0.0f32;
    let mut max_total_w = 0.0f32;
    let mut max_dps_w = 0.0f32;
    let mut max_name_w = 0.0f32;
    for (index, row) in rows[..dps.count].iter().enumerate() {
        let mut label_text = plugin_config::Text::<64>::new();
        let _ = write!(&mut label_text, "{}", row_label(row, config.dps_show_names));
        name_widths[index] = measure_text(&font, name_font, label_text.as_str());
        max_name_w = max_name_w.max(name_widths[index]);
        labels[index] = label_text;
        if config.dps_show_percent {
            let percent_tenths = if dps.total > 0 {
                row.damage * 1000 / dps.total
            } else {
                0
            };
            let mut text = plugin_config::Text::<16>::new();
            let _ = write!(
                &mut text,
                "{}.{}%",
                percent_tenths / 10,
                percent_tenths % 10
            );
            let size = if inline_stats { font_size } else { font_size * 0.9 };
            max_percent_w = max_percent_w.max(measure_text(&font, size, text.as_str()));
        }
        if inline_stats {
            let percent_tenths = if dps.total > 0 {
                row.damage * 1000 / dps.total
            } else {
                0
            };
            let dps_tenths = if dps.elapsed > 0.1 {
                (row.damage as f32 * 10.0 / dps.elapsed) as u64
            } else {
                0
            };
            let mut width = 0.0f32;
            let mut count = 0usize;
            if config.dps_show_dps {
                let mut text = plugin_config::Text::<24>::new();
                let _ = write!(&mut text, "{}.{}/s", dps_tenths / 10, dps_tenths % 10);
                let measured = measure_text(&font, font_size, text.as_str());
                width += measured;
                max_dps_w = max_dps_w.max(measured);
                count += 1;
            }
            if config.dps_show_damage {
                let mut text = plugin_config::Text::<24>::new();
                let _ = write!(&mut text, "{}", row.damage);
                let measured = measure_text(&font, font_size, text.as_str());
                width += measured;
                max_total_w = max_total_w.max(measured);
                count += 1;
            }
            if config.dps_show_percent {
                let mut text = plugin_config::Text::<16>::new();
                let _ = write!(
                    &mut text,
                    "{}.{}%",
                    percent_tenths / 10,
                    percent_tenths % 10
                );
                width += measure_text(&font, font_size, text.as_str());
                count += 1;
            }
            if count > 1 {
                width += stat_gap * (count - 1) as f32;
            }
            stats_widths[index] = width;
        }
    }

    let team_dps_tenths = if dps.elapsed > 0.1 {
        (dps.total as f32 * 10.0 / dps.elapsed) as u64
    } else {
        0
    };
    let mut team_total_text = plugin_config::Text::<32>::new();
    if config.dps_show_damage {
        let _ = write!(&mut team_total_text, "{}", dps.total);
    }
    let mut team_dps_text = plugin_config::Text::<24>::new();
    if config.dps_show_dps {
        let _ = write!(
            &mut team_dps_text,
            "{}.{}/s",
            team_dps_tenths / 10,
            team_dps_tenths % 10
        );
    }
    let show_team = config.dps_show_team && (config.dps_show_damage || config.dps_show_dps) && dps.count > 1;
    let team_total_w = if config.dps_show_damage {
        measure_text(&font, font_size, team_total_text.as_str())
    } else {
        0.0
    };
    let team_label_w = if config.dps_show_damage {
        measure_text(&font, name_font, "DMG")
    } else {
        0.0
    };
    let team_dps_w = if config.dps_show_dps {
        let size = if inline_stats || !config.dps_show_damage {
            font_size
        } else {
            small_font
        };
        measure_text(&font, size, team_dps_text.as_str())
    } else {
        0.0
    };
    let total_reserve_w = measure_text(&font, font_size, "99999");
    let total_col_w = total_reserve_w.max(max_total_w).max(team_total_w);
    let percent_reserve_w = measure_text(
        &font,
        if inline_stats { font_size } else { font_size * 0.9 },
        "100.0%",
    );

    let mut width = config.dps_curve_width * scale;
    if inline_stats {
        let mut group_width = 0.0f32;
        let mut group_count = 0usize;
        if config.dps_show_dps {
            group_width += max_dps_w;
            group_count += 1;
        }
        if config.dps_show_damage {
            group_width += total_col_w;
            group_count += 1;
        }
        if config.dps_show_percent {
            group_width += percent_reserve_w;
            group_count += 1;
        }
        if group_count > 1 {
            group_width += stat_gap * (group_count - 1) as f32;
        }
        let player_required = pad * 2.0 + icon_space + max_name_w
            + if group_width > 0.0 { stat_gap + group_width } else { 0.0 };
        width = width.max(player_required);
        if show_team {
            let mut team_required = pad * 2.0 + icon_space;
            if config.dps_show_damage {
                team_required += team_label_w + stat_gap + group_width;
            } else {
                team_required += group_width;
            }
            width = width.max(team_required);
        }
    }

    for m in &monsters.entries[..monsters.count] {
        let mut stats = plugin_config::Text::<64>::new();
        if config.show_hp {let _=write!(&mut stats,"{} / {}  {}%",m.hp,m.max_hp,m.hp*100/m.max_hp);}
        width=width.max(pad*2.0+12.0*scale+measure_text(&font,font_size,monster_label(m,config))+20.0*scale+measure_text(&font,font_size,stats.as_str()));
    }
    for m in &monsters.entries[..monsters.count] {
        if m.selected&&m.revealed&&m.charge.is_some_and(|c|c.points.is_none()) {width=width.max(pad*2.0+measure_text(&font,font_size*0.85,"Charged (value unavailable)"));}
        if let Some(snapshot)=m.parts {for p in &snapshot.parts {if p.visible(config.always_show_parts,config.show_breakable_hp,config.show_all_parts_hp){
            let mut label=plugin_config::Text::<128>::new();let _=write!(&mut label,"{}",if p.name.is_empty(){"Part"}else{p.name});
            if p.groups!=0{let _=write!(&mut label," · {}/{} broken",(p.groups&snapshot.breaks).count_ones(),p.groups.count_ones());}else if p.severable(){let _=write!(&mut label," · SEVERABLE");}
            width=width.max(pad*2.0+measure_text(&font,font_size*0.65,label.as_str())+165.0*scale);
        }}}
    }
    if config.show_weakness {
        for m in &monsters.entries[..monsters.count] {
            let mut w=pad*2.0;
            for (i,label) in ["FIRE","WATER","ICE","THUNDER","DRAGON"].iter().enumerate(){
                if m.weakness & (1<<i)!=0 {w+=measure_text(&font,font_size*0.75,label)+23.0*scale;}
            }
            width=width.max(w);
        }
    }
    let header_rows = if has_dps && config.dps_show_timer { 1.0 } else { 0.0 };
    let team_top_shift = if show_team { 4.0 * scale } else { 0.0 };
    let team_row_h = if !show_team {
        0.0
    } else if two_line && config.dps_show_damage && config.dps_show_dps {
        two_line_bottom - team_top_shift
    } else {
        name_bottom - team_top_shift
    };
    let bottom_pad = if show_team { 3.0 * scale } else { pad };
    let height = monster_height + pad
        + bottom_pad
        + header_rows * header_h
        + chart_height
        + first_row_gap
        + row_h * dps.count as f32
        + team_row_h;
    // Measure first, then fit the complete panel. Never move its bottom offscreen.
    let fit=layout::fit_scale(display.width,display.height,width,height,config.dps_offset_y*scale);
    if fit<0.999 && attempt<4 {
        return draw_panel_scaled(dps,config,display,monsters,scale*fit*0.99,attempt+1);
    }
    let x = (config.dps_offset_x * scale).clamp(0.0, (display.width - width).max(0.0));
    let y = layout::top(display.height, height, config.dps_offset_y * scale);
    let Some(draw) = foreground_draw_list() else { return; };
    let (bg,accent,fg,rounding)=match config.theme {
        1 => ([34,25,19],[213,162,79],[242,216,170],6.0),
        2 => ([6,10,17],[111,216,255],[244,247,252],3.0),
        3 => ([0,0,0],[220,220,220],[255,255,255],2.0),
        _ => ([50,25,24],[236,212,44],[242,238,232],0.0),
    };
    let gold = imgui::color(accent[0],accent[1],accent[2],255);
    let white = imgui::color(fg[0],fg[1],fg[2],255);

    let alpha = config.dps_background_alpha.clamp(0.0, 255.0) as u8;
    if alpha > 0 {
        draw.add_rect_filled(
            ImVec2::new(x, y),
            ImVec2::new(x + width, y + height),
            imgui::color(bg[0], bg[1], bg[2], alpha),
            rounding * scale,
        );
    }

    if config.theme==0 && alpha>0 {
        for edge in [y,y+height-3.4*scale] {
            draw.add_rect_filled(ImVec2::new(x,edge),ImVec2::new(x+width,edge+2.0*scale),imgui::color(176,34,30,alpha),0.0);
            draw.add_rect_filled(ImVec2::new(x,edge+2.0*scale),ImVec2::new(x+width,edge+3.4*scale),imgui::color(80,142,54,alpha),0.0);
        }
    }
    let mut text_y = y + pad - 2.0 * scale;
    // One health line + thin bar. Only active statuses occupy a second line.
    for m in &monsters.entries[..monsters.count] {
        let mut stats = plugin_config::Text::<64>::new();
        let _ = write!(&mut stats, "{} / {}  {}%",m.hp,m.max_hp,m.hp*100/m.max_hp);
        let mut name_text=plugin_config::Text::<96>::new();
        if m.selected {let _=write!(&mut name_text,"> ");}
        let _=write!(&mut name_text,"{}",monster_label(m,config));
        let name=name_text.as_str();
        draw.add_text_with_font(&font,font_size,ImVec2::new(x+pad,text_y),gold,name);
        if config.show_hp {
        let sw=measure_text(&font,font_size,stats.as_str());
        draw.add_text_with_font(&font,font_size,ImVec2::new(x+width-pad-sw,text_y),white,stats.as_str());
        let by=text_y+font_size+5.0*scale;
        let bw=width-2.0*pad;
        draw.add_rect_filled(ImVec2::new(x+pad,by),ImVec2::new(x+width-pad,by+4.0*scale),imgui::color(255,255,255,30),1.0*scale);
        draw.add_rect_filled(ImVec2::new(x+pad,by),ImVec2::new(x+pad+bw*m.hp as f32/m.max_hp as f32,by+4.0*scale),gold,1.0*scale);
        }
        text_y+=monster_row_h;
        if config.show_location {
            let mut line=plugin_config::Text::<48>::new();
            let _=write!(&mut line,"Area {}",m.room);
            draw.add_text_with_font(&font,font_size*0.85,ImVec2::new(x+pad,text_y-5.0*scale),white,line.as_str());
            text_y+=status_h;
        }
        if config.show_weakness && m.weakness!=0 {
            let mut bx=x+pad;
            for (i,(label,rgb)) in [("FIRE",[240,93,61]),("WATER",[77,160,245]),("ICE",[126,227,240]),("THUNDER",[245,218,83]),("DRAGON",[195,116,225])].iter().enumerate() {
                if m.weakness & (1<<i)==0 {continue;}
                let w=measure_text(&font,font_size*0.75,label)+19.0*scale;
                draw.add_rect_filled(ImVec2::new(bx,text_y-4.0*scale),ImVec2::new(bx+w,text_y+9.0*scale),imgui::color(rgb[0],rgb[1],rgb[2],60),2.0*scale);
                let pixels:[[u8;7];5]=[
                    [8,12,28,30,63,63,30], // flame
                    [8,8,28,28,62,62,28], // water drop
                    [73,42,28,127,28,42,73], // snowflake
                    [6,12,24,62,12,24,16], // lightning
                    [65,99,54,28,54,34,65], // dragon crest
                ];
                for (py,bits) in pixels[i].iter().enumerate(){for px in 0..7 {
                    if bits & (1<<px)!=0 {
                        let ix=bx+(3.0+px as f32)*scale;let iy=text_y+(-2.0+py as f32)*scale;
                        draw.add_rect_filled(ImVec2::new(ix,iy),ImVec2::new(ix+scale,iy+scale),imgui::color(rgb[0],rgb[1],rgb[2],255),0.0);
                    }
                }}
                draw.add_text_with_font(&font,font_size*0.75,ImVec2::new(bx+15.0*scale,text_y-4.0*scale),imgui::color(rgb[0],rgb[1],rgb[2],255),label);
                bx+=w+4.0*scale;
            }
            text_y+=status_h;
        }
        if let Some(snapshot)=m.parts {
            for (slot,p) in snapshot.parts.iter().enumerate(){
                if !p.visible(config.always_show_parts,config.show_breakable_hp,config.show_all_parts_hp){continue;}
                let bars=config.show_all_parts_hp||(config.show_breakable_hp&&p.breakable());
                let mut line=plugin_config::Text::<128>::new();
                if p.name.is_empty(){let _=write!(&mut line,"Part {}",slot+1);}else{let _=write!(&mut line,"{}",p.name);}
                let broken=p.groups&snapshot.breaks;
                if p.severed(){let _=write!(&mut line," · SEVERED");}
                else if p.groups!=0 {let _=write!(&mut line," · {}/{} broken",broken.count_ones(),p.groups.count_ones());}
                else if p.severable(){let _=write!(&mut line," · severable");}
                draw.add_text_with_font(&font,font_size*0.65,ImVec2::new(x+pad,text_y-5.0*scale),white,line.as_str());
                if bars {
                    let left=x+width-pad-145.0*scale;let right=x+width-pad;
                    draw.add_rect_filled(ImVec2::new(left,text_y+8.0*scale),ImVec2::new(right,text_y+11.0*scale),imgui::color(60,57,48,220),0.0);
                    if p.max>0 {let ratio=(p.hp as f32/p.max as f32).clamp(0.0,1.0);draw.add_rect_filled(ImVec2::new(left,text_y+8.0*scale),ImVec2::new(left+(right-left)*ratio,text_y+11.0*scale),imgui::color(222,192,77,255),0.0);}
                    let mut stats=plugin_config::Text::<64>::new();if p.max>0{let _=write!(&mut stats,"{}/{} · {} flinches",p.hp,p.max,p.flinches);}else{let _=write!(&mut stats,"{} HP · {} flinches",p.hp,p.flinches);}
                    draw.add_text_with_font(&font,font_size*0.6,ImVec2::new(left,text_y-5.0*scale),white,stats.as_str());
                }
                text_y+=status_h;
                if bars&&p.secondary(){
                    let mut line=plugin_config::Text::<96>::new();let label=if p.severable(){"Sever"}else{"Secondary break"};
                    let _=write!(&mut line,"  {}: {} HP · {}/{}",label,p.sever_hp,p.sever_count,p.sever_required);
                    draw.add_text_with_font(&font,font_size*0.65,ImVec2::new(x+pad,text_y-5.0*scale),white,line.as_str());
                    if p.sever_max>0{let left=x+width-pad-145.0*scale;let right=x+width-pad;let ratio=(p.sever_hp as f32/p.sever_max as f32).clamp(0.0,1.0);draw.add_rect_filled(ImVec2::new(left,text_y+4.0*scale),ImVec2::new(right,text_y+7.0*scale),imgui::color(60,57,48,220),0.0);draw.add_rect_filled(ImVec2::new(left,text_y+4.0*scale),ImVec2::new(left+(right-left)*ratio,text_y+7.0*scale),imgui::color(115,181,199,255),0.0);}
                    text_y+=status_h;
                }
            }
        }
        if !m.selected || !m.revealed {continue;}
        let mut tokens: [plugin_config::Text<32>;8] = core::array::from_fn(|_|plugin_config::Text::new());
        let mut n=0;
        if m.rage>0.0 {let _=write!(&mut tokens[n],"RAGE {}s",(m.rage/60.0+0.999) as u32);n+=1;}
        for (i,&(v,max)) in m.ailments.iter().enumerate(){if status_events::buildup_visible(i,(v,max)) {
            let _=write!(&mut tokens[n],"{} {}%",["PSN","SLP","PAR","KO","EXH","MNT","BLST"][i],v as i32*100/max as i32);n+=1;
        }}
        if let Some(charge)=m.charge {
            let mut label=plugin_config::Text::<64>::new();
            let state=if charge.active{"Charged"}else{"Charge"};
            if let Some(points)=charge.points {let _=write!(&mut label,"{} {}/100",state,points);}else{let _=write!(&mut label,"{} (value unavailable)",state);}
            let color=if charge.active{imgui::color(115,181,235,255)}else{imgui::color(178,186,196,255)};
            draw.add_text_with_font(&font,font_size*0.85,ImVec2::new(x+pad,text_y-5.0*scale),color,label.as_str());
            if let Some(points)=charge.points {
                let left=x+width*0.60;let right=x+width-pad;
                draw.add_rect_filled(ImVec2::new(left,text_y+1.0*scale),ImVec2::new(right,text_y+4.0*scale),imgui::color(62,68,76,255),0.0);
                draw.add_rect_filled(ImVec2::new(left,text_y+1.0*scale),ImVec2::new(left+(right-left)*f32::from(points)/100.0,text_y+4.0*scale),color,0.0);
            }
            text_y+=status_h;
        }
        for group in tokens[..n].chunks(4) {
            for (col,token) in group.iter().enumerate() {
                draw.add_text_with_font(&font,font_size*0.85,ImVec2::new(x+pad+col as f32*(width-pad*2.0)/4.0,text_y-5.0*scale),imgui::color(178,186,196,255),token.as_str());
            }
            text_y+=status_h;
        }
    }

    if has_dps && config.dps_show_timer {
        let mut header = plugin_config::Text::<64>::new();
        let seconds = dps.elapsed as u32;
        let _ = write!(&mut header, "DPS  {:02}:{:02}", seconds / 60, seconds % 60);
        draw.add_text_with_font(
            &font,
            font_size,
            ImVec2::new(x + pad, text_y),
            gold,
            header.as_str(),
        );
        text_y += header_h;
    }

    let chart_bottom = text_y + chart_height;
    if show_chart {
    let chart_left = x + pad;
    let chart_width = width - pad * 2.0;
    let mut max_damage = 1u32;
    for index in 0..dps.history_len {
        let sample = &dps.history[(history_start(dps) + index) % HISTORY_CAP];
        for value in sample.damage {
            max_damage = max_damage.max(value);
        }
    }
    for step in 1..=3 {
        let line_y = chart_bottom - chart_height * step as f32 / 4.0;
        draw.add_line(
            ImVec2::new(chart_left, line_y),
            ImVec2::new(chart_left + chart_width, line_y),
            imgui::color(255, 255, 255, 32),
            1.0 * scale,
        );
    }

    let start = history_start(dps);
    let first_time = if dps.history_len > 0 {
        dps.history[start].time
    } else {
        0.0
    };
    let last_time = if dps.history_len > 0 {
        dps.history[(start + dps.history_len - 1) % HISTORY_CAP].time
    } else {
        0.0
    };
    let time_span = (last_time - first_time).max(1.0);
    if config.dps_curve_fill {
        for (row_index, row) in rows[..dps.count].iter().enumerate() {
            let color_index = row_color(row, row_index);
            let mut points: [ImVec2; HISTORY_CAP] =
                core::array::from_fn(|_| ImVec2::new(0.0, 0.0));
            for index in 0..dps.history_len {
                let sample = &dps.history[(start + index) % HISTORY_CAP];
                let px = if dps.history_len == 1 {
                    chart_left + chart_width
                } else {
                    chart_left
                        + ((sample.time - first_time) / time_span).clamp(0.0, 1.0)
                            * chart_width
                };
                let value = sample.damage[color_index] as f32 / max_damage as f32;
                let py = chart_bottom - value.clamp(0.0, 1.0) * chart_height;
                points[index] = ImVec2::new(px, py);
            }
            let point_count = dps.history_len;
            if point_count > 1 {
                let fill = dps_color(color_index, 55);
                let step = 1.0 * scale;
                let right = chart_left + chart_width;
                let mut column = chart_left;
                let mut segment = 1usize;
                while column < right {
                    let next = (column + step).min(right);
                    while segment < point_count && points[segment].x < column {
                        segment += 1;
                    }
                    let y = if column <= points[0].x || segment == 0 {
                        points[0].y
                    } else if segment >= point_count {
                        points[point_count - 1].y
                    } else {
                        let previous = points[segment - 1];
                        let point = points[segment];
                        if point.x > previous.x {
                            previous.y
                                + (point.y - previous.y)
                                    * ((column - previous.x) / (point.x - previous.x))
                                        .clamp(0.0, 1.0)
                        } else {
                            point.y
                        }
                    };
                    draw.add_rect_filled(
                        ImVec2::new(column, y),
                        ImVec2::new(next, chart_bottom),
                        fill,
                        0.0,
                    );
                    column = next;
                }
            }
        }
    }

    for (row_index, row) in rows[..dps.count].iter().enumerate() {
        let color_index = row_color(row, row_index);
        let mut points: [ImVec2; HISTORY_CAP] =
            core::array::from_fn(|_| ImVec2::new(0.0, 0.0));
        for index in 0..dps.history_len {
            let sample = &dps.history[(start + index) % HISTORY_CAP];
            let px = if dps.history_len == 1 {
                chart_left + chart_width
            } else {
                chart_left
                    + ((sample.time - first_time) / time_span).clamp(0.0, 1.0)
                        * chart_width
            };
            let value = sample.damage[color_index] as f32 / max_damage as f32;
            let py = chart_bottom - value.clamp(0.0, 1.0) * chart_height;
            points[index] = ImVec2::new(px, py);
        }
        let point_count = dps.history_len;
        let color = dps_color(color_index, 230);
        if point_count > 1 {
            draw.add_polyline(
                &points[..point_count],
                color,
                2.0 * scale,
                ImDrawFlags::None,
            );
        } else if point_count == 1 {
            draw.add_circle_filled(points[0], 2.0 * scale, color, 12);
        }
    }

    }

    let mut row_y = chart_bottom + first_row_gap;
    let right = x + width - pad;
    for (row_index, row) in rows[..dps.count].iter().enumerate() {
        let color_index = row_color(row, row_index);
        let percent_tenths = if dps.total > 0 {
            row.damage * 1000 / dps.total
        } else {
            0
        };
        let dps_tenths = if dps.elapsed > 0.1 {
            (row.damage as f32 * 10.0 / dps.elapsed) as u64
        } else {
            0
        };

        let icon_x = x + pad;
        let icon_bottom =
            row_y + row_text_offset + name_font - 2.0 * scale + 1.0 * scale;
        if config.dps_show_weapon {
            let icon_index = row_icon(row);
            if icon_index != 0xFF {
                let icon = &icons::ICONS[icon_index];
                if icon.width != 0 && icon.height != 0 {
                    let icon_bottom = if icon_index >= PALICO_ICON {
                        icon_bottom - PALICO_ICON_LIFT * scale
                    } else {
                        icon_bottom
                    };
                    draw_pixel_icon(
                        &draw,
                        icon,
                        icon_x,
                        icon_bottom,
                        icon_size,
                        dps_color(color_index, 210),
                    );
                }
            }
        }
        let name_color = if row.local { gold } else { white };
        let name_x = x + pad + icon_space;
        draw.add_text_with_font(
            &font,
            name_font,
            ImVec2::new(name_x, row_y + row_text_offset),
            name_color,
            labels[row_index].as_str(),
        );
        if config.dps_show_hits {
            let mut hit_text = plugin_config::Text::<16>::new();
            let _ = write!(&mut hit_text, "  {}h", row.hits);
            let name_baseline = row_y + row_text_offset + font_ascent(&font, name_font);
            let hit_y = name_baseline - font_ascent(&font, font_size);
            draw.add_text_with_font(
                &font,
                font_size,
                ImVec2::new(name_x + name_widths[row_index] + 2.0 * scale, hit_y),
                name_color,
                hit_text.as_str(),
            );
        }

        if config.dps_show_percent_bar {
            let bar_x = x + pad;
            let bar_w = width - pad * 2.0;
            let bar_y = if two_line {
                row_y + font_size + small_font - 1.0 * scale
            } else {
                row_y + name_bottom + 1.0 * scale
            };
            draw.add_rect_filled(
                ImVec2::new(bar_x, bar_y),
                ImVec2::new(bar_x + bar_w, bar_y + 3.0 * scale),
                imgui::color(255, 255, 255, 35),
                1.0 * scale,
            );
            let fill_w = bar_w * percent_tenths as f32 / 1000.0;
            if fill_w > 0.0 {
                draw.add_rect_filled(
                    ImVec2::new(bar_x, bar_y),
                    ImVec2::new(bar_x + fill_w, bar_y + 3.0 * scale),
                    dps_color(color_index, 220),
                    1.0 * scale,
                );
            }
        }

        let mut damage_text = plugin_config::Text::<24>::new();
        if config.dps_show_damage {
            let _ = write!(&mut damage_text, "{}", row.damage);
        }
        let mut dps_text = plugin_config::Text::<24>::new();
        if config.dps_show_dps {
            let _ = write!(
                &mut dps_text,
                "{}.{}/s",
                dps_tenths / 10,
                dps_tenths % 10
            );
        }
        let mut percent_text = plugin_config::Text::<16>::new();
        if config.dps_show_percent {
            let _ = write!(
                &mut percent_text,
                "{}.{}%",
                percent_tenths / 10,
                percent_tenths % 10
            );
        }
        let baseline = row_y + row_text_offset + font_ascent(&font, name_font);
        let inline_y = baseline - font_ascent(&font, font_size);
        let percent_column_w = if inline_stats {
            percent_reserve_w
        } else {
            max_percent_w
        };
        let stats_right = right
            - if config.dps_show_percent {
                percent_column_w + stat_gap
            } else {
                0.0
            };
        if inline_stats {
            let dps_w = if config.dps_show_dps {
                measure_text(&font, font_size, dps_text.as_str())
            } else {
                0.0
            };
            let percent_w = if config.dps_show_percent {
                measure_text(&font, font_size, percent_text.as_str())
            } else {
                0.0
            };
            let total_left = stats_right - total_col_w;
            let damage_x = total_left;
            let dps_right = if config.dps_show_damage {
                total_left - stat_gap
            } else {
                stats_right
            };
            let dps_x = dps_right - dps_w;
            let percent_x = right - percent_w;
            if config.dps_show_dps {
                draw.add_text_with_font(
                    &font,
                    font_size,
                    ImVec2::new(dps_x, inline_y),
                    white,
                    dps_text.as_str(),
                );
            }
            if config.dps_show_damage {
                draw.add_text_with_font(
                    &font,
                    font_size,
                    ImVec2::new(damage_x, inline_y),
                    white,
                    damage_text.as_str(),
                );
            }
            if config.dps_show_percent {
                draw.add_text_with_font(
                    &font,
                    font_size,
                    ImVec2::new(percent_x, inline_y),
                    white,
                    percent_text.as_str(),
                );
            }
        } else {
            let percent_w = if config.dps_show_percent {
                measure_text(&font, font_size * 0.9, percent_text.as_str())
            } else {
                0.0
            };
            let numbers_right = stats_right;
            let both_numbers = config.dps_show_damage && config.dps_show_dps;
            if config.dps_show_damage {
                let damage_w = measure_text(&font, font_size, damage_text.as_str());
                let damage_y = if both_numbers {
                    row_y
                } else {
                    baseline - font_ascent(&font, font_size)
                };
                draw.add_text_with_font(
                    &font,
                    font_size,
                    ImVec2::new(numbers_right - damage_w, damage_y),
                    white,
                    damage_text.as_str(),
                );
            }
            if config.dps_show_dps {
                let dps_size = if both_numbers { small_font } else { font_size };
                let dps_w = measure_text(&font, dps_size, dps_text.as_str());
                let dps_y = if both_numbers {
                    row_y + font_size * 0.92 - 2.0 * scale
                } else {
                    baseline - font_ascent(&font, dps_size)
                };
                draw.add_text_with_font(
                    &font,
                    dps_size,
                    ImVec2::new(numbers_right - dps_w, dps_y),
                    white,
                    dps_text.as_str(),
                );
            }
            if config.dps_show_percent {
                let percent_size = font_size * 0.9;
                let percent_y = baseline - font_ascent(&font, percent_size);
                draw.add_text_with_font(
                    &font,
                    percent_size,
                    ImVec2::new(right - percent_w, percent_y),
                    white,
                    percent_text.as_str(),
                );
            }
        }
        row_y += row_h;
    }

    if show_team {
        let team_y = row_y - team_top_shift;
        let team_red = imgui::color(235, 60, 60, 255);
        let team_right = right
            - if config.dps_show_percent {
                (if inline_stats {
                    percent_reserve_w
                } else {
                    max_percent_w
                }) + stat_gap
            } else {
                0.0
            };
        let inline_total_left = team_right - total_col_w;
        let inline_dps_right = if config.dps_show_damage {
            inline_total_left - stat_gap
        } else {
            team_right
        };
        let team_total_x = if inline_stats {
            inline_total_left
        } else {
            team_right - team_total_w
        };
        let team_baseline = team_y + row_text_offset + font_ascent(&font, name_font);
        let team_total_y = if inline_stats {
            team_baseline - font_ascent(&font, font_size)
        } else if config.dps_show_dps {
            team_y
        } else {
            team_baseline - font_ascent(&font, font_size)
        };
        if config.dps_show_damage {
            draw.add_text_with_font(
                &font,
                name_font,
                ImVec2::new(x + pad + icon_space, team_y + row_text_offset),
                team_red,
                "DMG",
            );
            draw.add_text_with_font(
                &font,
                font_size,
                ImVec2::new(team_total_x, team_total_y),
                team_red,
                team_total_text.as_str(),
            );
        }
        if config.dps_show_dps {
            let team_dps_size = if inline_stats || !config.dps_show_damage {
                font_size
            } else {
                small_font
            };
            let team_dps_y = if inline_stats || !config.dps_show_damage {
                team_baseline - font_ascent(&font, team_dps_size)
            } else {
                team_y + font_size * 0.92 - 2.0 * scale
            };
            let team_dps_x = if inline_stats {
                inline_dps_right - team_dps_w
            } else {
                team_right - team_dps_w
            };
            draw.add_text_with_font(
                &font,
                team_dps_size,
                ImVec2::new(team_dps_x, team_dps_y),
                team_red,
                team_dps_text.as_str(),
            );
        }
    }
}

fn history_start(_dps: &DpsState) -> usize {
    0
}

fn update_timers(base:u32,monsters:&monsters::Snapshot){
 MAP_FRAME.fetch_add(1,Ordering::Relaxed);
 let mut out=timers::Snapshot::EMPTY;
 let memory=monsters::GameMemory{base};
 if TIMERS_AVAILABLE.load(Ordering::Relaxed)&&matches!(quest::capture(&memory,base),quest::Observation::Quest{..}){
  if let Some(player)=timers::local_player(&memory,base){out.drinks=timers::drinks(&memory,player,timers::temperature(&memory,base,player));}
  for (i,m) in monsters.entries[..monsters.count].iter().enumerate(){
   if m.hp<=0{continue;}
   if let Some(seconds)=timers::paint(&memory,m.object){
    out.paint[out.count]=timers::Paint{seconds,species:m.species,revealed:m.revealed,number:(i+1) as u8};out.count+=1;
   }
  }
 }
 TIMERS.with(|s|*s=out);
}

fn draw_debuff_timers(c:&Config){
 if !c.timers_debuffs&&!c.timers_drinks{return;}
 let base=GAME_BASE.load(Ordering::Relaxed);let memory=monsters::GameMemory{base};
 let icons=DEBUFF_ICONS.with(|s|*s);let frame=MAP_FRAME.load(Ordering::Relaxed);
 let io=imgui::get_io();let w=io.DisplaySize.x;let h=io.DisplaySize.y;
 if w<=0.0||h<=0.0{return;}
 let scale=(w/1280.0).min(h/720.0);
 let font=unsafe{CUSTOM_FONT}.unwrap_or_else(imgui::ImFont::current);let size=14.0*scale;
 let Some(draw)=foreground_draw_list()else{return;};
 for icon in icons{
  if icon.node==0||frame.wrapping_sub(icon.frame)>4{continue;}
  let seconds=if debuffs::drink(icon).is_some(){
   if !c.timers_drinks{continue;}debuffs::drink_remaining(&memory,base,icon)
  }else{if !c.timers_debuffs{continue;}debuffs::remaining(&memory,icon)};
  let Some(seconds)=seconds else{continue;};
  let Some((x,y))=map_timers::position(&memory,icon.node)else{continue;};
  let mut value=plugin_config::Text::<16>::new();let _=write!(value,"{}:{:02}",seconds/60,seconds%60);
  let width=measure_text(&font,size,value.as_str());
  let pos=ImVec2::new((w-1280.0*scale)*0.5+x*scale-width*0.5,(h-720.0*scale)*0.5+y*scale-size*0.5);
  for (dx,dy) in [(-1.0,0.0),(1.0,0.0),(0.0,-1.0),(0.0,1.0)]{draw.add_text_with_font(&font,size,ImVec2::new(pos.x+dx*scale,pos.y+dy*scale),imgui::color(0,0,0,255),value.as_str());}
  draw.add_text_with_font(&font,size,pos,imgui::color(255,255,255,255),value.as_str());
 }
}

fn draw_map_timers(c:&Config){
 if !c.timers_paint{return;}
 let base=GAME_BASE.load(Ordering::Relaxed);let memory=monsters::GameMemory{base};
 let markers=MAP_MARKERS.with(|s|*s);let frame=MAP_FRAME.load(Ordering::Relaxed);
 let monsters=MONSTERS.with(|s|*s);
 let io=imgui::get_io();let w=io.DisplaySize.x;let h=io.DisplaySize.y;
 if w<=0.0||h<=0.0{return;}
 let scale=(w/1280.0).min(h/720.0);
 let font=unsafe{CUSTOM_FONT}.unwrap_or_else(imgui::ImFont::current);let size=14.0*scale;
 let Some(draw)=foreground_draw_list() else{return;};
 for marker in markers{
  if marker.node==0||frame.wrapping_sub(marker.frame)>4{continue;}
  if !monsters.entries[..monsters.count].iter().any(|m|m.object==marker.object&&m.hp>0){continue;}
  let Some(seconds)=timers::paint(&memory,marker.object) else{continue;};
  let Some((x,y))=map_timers::position(&memory,marker.node) else{continue;};
  let mut value=plugin_config::Text::<16>::new();let _=write!(value,"{}:{:02}",seconds/60,seconds%60);
  let width=measure_text(&font,size,value.as_str());
  let pos=ImVec2::new((w-1280.0*scale)*0.5+x*scale-width*0.5,(h-720.0*scale)*0.5+y*scale-size*0.5);
  for (dx,dy) in [(-1.0,0.0),(1.0,0.0),(0.0,-1.0),(0.0,1.0)]{
   draw.add_text_with_font(&font,size,ImVec2::new(pos.x+dx*scale,pos.y+dy*scale),imgui::color(0,0,0,255),value.as_str());
  }
  let color=if seconds<=30{imgui::color(255,145,110,255)}else{imgui::color(255,255,255,255)};
  draw.add_text_with_font(&font,size,pos,color,value.as_str());
 }
}

fn draw_timers(){
 let c=STATE.with(|s|s.config);
 if !c.timers_enabled||!TIMERS_AVAILABLE.load(Ordering::Relaxed){return;}
 draw_map_timers(&c);
 draw_debuff_timers(&c);
}

fn draw_dps() {
    let mut dps = DPS.with(|state| *state);
    let mut monsters = MONSTERS.with(|state| *state);
    let focus=monsters.entries[..monsters.count].iter().find(|m|m.selected).or_else(||if monsters.count==1{Some(&monsters.entries[0])}else{None}).map(|m|m.object);
    if let Some(focus)=focus {dps=MONSTER_DPS.with(|pool|pool.keys.iter().position(|key|*key==focus).map(|i|pool.states[i]).unwrap_or_else(DpsState::new));}
    if !dps.visible || !dps.active || dps.total == 0 { dps.count=0; }
    if monsters.count==0 && dps.count==0 && forge::graphics::Graphics::is_menu_visible() {
        monsters.count=1;
        monsters.entries[0]=monsters::Monster {selected:true,revealed:true,room:3,weakness:8,species:2,hp:3850,max_hp:6000,rage:2520.0,ailments:[(60,150),(0,0),(0,0),(0,0),(0,0),(0,0),(0,0)],..monsters::Monster::default()};
        dps.count=1;dps.total=2150;dps.elapsed=42.0;
        dps.owners[0]=OwnerDps {local:true,damage:2150,slot:0,color:0,weapon:0,..OwnerDps::EMPTY};
    }
    if monsters.count==0 && dps.count==0 {return;}
    let state = STATE.with(|state| state.snapshot());
    if !state.config.enabled { return; }
    if !state.config.show_dps {dps.count=0;}
    if state.config.target_only {
        let mut n=0;
        for i in 0..monsters.count {if monsters.entries[i].selected {monsters.entries[n]=monsters.entries[i];n+=1;}}
        monsters.count=n;
    }
    if monsters.count==0 && dps.count==0 {return;}
    // Recomp renders ImGui at its desktop viewport size, unlike Switch crop coordinates.
    let io=imgui::get_io();
    let display=DisplayArea {width:io.DisplaySize.x,height:io.DisplaySize.y};
    if display.width>0.0 && display.height>0.0 {draw_dps_curve(&dps, &state.config, display, &monsters);}
}

#[forge::imgui_render]
fn draw_settings() {
    // Forge 共用一个 Plugin UI 窗口；折叠标题不会替内部控件建立 ID 作用域。
    imgui::push_id(imgui::Id::Str(c"mhgu_dps_meter"));
    if imgui::tree_node(c"HuntLog") {
        draw_settings_body();
        imgui::tree_pop();
    }
    imgui::pop_id();
}

fn draw_settings_body() {
    let (status,run,lost)=hunt_runtime::status();
    let mut recorder_status=plugin_config::Text::<192>::new();
    let _=write!(recorder_status,"Background history: {} | run {} | dropped {}",status,run,lost);
    imgui::text(recorder_status.as_str());
    imgui::text("History records even with the HUD disabled. File: hunter_hunts.jsonl");
    imgui::text("DPS follows the selected large monster; small monsters are excluded.");
    imgui::text_wrapped(c"Follows your target-camera selection. Spoiler options are opt-in.");
    imgui::text_wrapped(c"Town preview shows example monster and DPS data while this menu is open.");
    if !INSTALLED.load(Ordering::Acquire) {
        imgui::text("DPS hooks unavailable. Monster HUD remains available; see Forge log.");
    }
    MENU.with(|edit| {
        let c = &mut edit.draft;
        imgui::checkbox(c"Enable monster HP / DPS panel", &mut c.enabled);
        if imgui::tree_node(c"Timers (independent widget)") {
            imgui::checkbox(c"Show timers", &mut c.timers_enabled);
            imgui::checkbox(c"Paintball countdown on map", &mut c.timers_paint);
            imgui::checkbox(c"Hot / Cool Drink countdowns on icons", &mut c.timers_drinks);
            imgui::checkbox(c"Debuff countdowns on icons", &mut c.timers_debuffs);
            imgui::text_wrapped(c"Independent of monster HP and DPS. Reads active effects from the game. Drink times adjust to your current area's temperature.");
            if !TIMERS_AVAILABLE.load(Ordering::Relaxed){imgui::text("Timers unavailable: game code did not match MHGU 1.4.");}
            imgui::tree_pop();
        }
        if imgui::button(c"Reset DPS", Default::default()) {
            DPS.with(|dps| dps.reset_now());
            MONSTER_DPS.with(|pool|{for state in &mut pool.states{state.reset_now();}});
        }
        if imgui::tree_node(c"Spoiler options") {
            imgui::checkbox(c"Show location", &mut c.show_location);
            imgui::checkbox(c"Show name even if undiscovered", &mut c.show_undiscovered_name);
            imgui::checkbox(c"Show target monster only", &mut c.target_only);
            imgui::checkbox(c"Show weakness icons", &mut c.show_weakness);
            imgui::checkbox(c"Show breakable parts", &mut c.always_show_parts);
            imgui::checkbox(c"Show HP for breakable parts", &mut c.show_breakable_hp);
            imgui::checkbox(c"Show HP for all parts", &mut c.show_all_parts_hp);
            imgui::checkbox(c"Show HP", &mut c.show_hp);
            imgui::text_wrapped(c"Weakness badges show strongest elements across verified live part slots; unavailable for siege monsters. Part bars show live stagger HP; break flags and sever progress are separate. These options can reveal undiscovered monster details.");
            imgui::tree_pop();
        }
        imgui::checkbox(c"Show DPS meter", &mut c.show_dps);
        if imgui::tree_node(c"Resize and position") {
            plugin_config::number(c"HUD scale (1 = original)", &mut c.hud_scale, 0.5, 2.0);
            plugin_config::number(c"Panel width", &mut c.dps_curve_width, 220.0, 600.0);
            plugin_config::number(c"Panel X", &mut c.dps_offset_x, 0.0, 1280.0);
            plugin_config::number(c"Bottom margin", &mut c.dps_offset_y, 0.0, 720.0);
            imgui::text("Bottom anchored; additional rows grow upward.");
            imgui::tree_pop();
        }
        if imgui::tree_node(c"Theme") {
            for (i,label) in [c"Hunter (Recomp default)",c"Gold",c"Blue",c"Minimal"].iter().enumerate() {
                if imgui::radio_button(label,c.theme==i as u8) {c.theme=i as u8;}
            }
            imgui::tree_pop();
        }
        if imgui::tree_node(c"DPS details and appearance") {
        plugin_config::number(
            c"DPS background alpha",
            &mut c.dps_background_alpha,
            0.0,
            255.0,
        );
        imgui::checkbox(c"Show timer", &mut c.dps_show_timer);
        imgui::checkbox(c"Show damage", &mut c.dps_show_damage);
        imgui::checkbox(c"Show percent", &mut c.dps_show_percent);
        imgui::checkbox(c"Show DPS", &mut c.dps_show_dps);
        imgui::checkbox(c"Show hits", &mut c.dps_show_hits);
        imgui::checkbox(c"Show chart", &mut c.dps_show_curve);
        plugin_config::number(c"Curve height", &mut c.dps_curve_height, 55.0, 200.0);
        imgui::checkbox(c"Curve fill", &mut c.dps_curve_fill);
        imgui::checkbox(c"Percent bar", &mut c.dps_show_percent_bar);
        imgui::checkbox(c"Weapon icons", &mut c.dps_show_weapon);
        imgui::checkbox(c"Player names", &mut c.dps_show_names);
        imgui::checkbox(c"Inline stats", &mut c.dps_inline_stats);
        imgui::checkbox(c"Team summary", &mut c.dps_show_team);
        imgui::tree_pop();
        }
        let mut input = FONT_INPUT.with(|b| *b);
        if imgui::input_text(c"DPS font TTF filename", &mut input, ImGuiInputTextFlags::None) {
            FONT_INPUT.with(|b| *b = input);
            edit.message = "";
            if let Ok(name) = plugin_config::input_text(&input) {
                if name == "default" {
                    c.font_file = None;
                } else if let Some(file) = FontFile::new(name) {
                    c.font_file = Some(file);
                }
            }
        }
        imgui::text("Menu changes preview live. Font changes need save and restart.");
        let reset = plugin_config::footer(
            edit,
            Config::DEFAULT,
            &plugin_config::DAMAGE,
            false,
            |config| {
                let name = plugin_config::input_text(&input)?;
                if name != "default" && FontFile::new(name).is_none() {
                    return Err("Font must be default or a .ttf filename without a directory.");
                }
                let mut text = plugin_config::Text::<CONFIG_MAX_BYTES>::new();
                config
                    .write_ini(&mut text)
                    .map_err(|_| "Configuration too large.")?;
                Config::parse(text.as_str()).map_err(|e| e.message)?;
                plugin_config::DAMAGE.save(text.as_str())
            },
        );
        if reset {
            set_font_input(edit.draft);
        }
        let config = edit.draft;
        let changed = ENABLED.swap(config.enabled, Ordering::Relaxed) != config.enabled;
        STATE.with(|state| {
            if changed {
                state.clear();
            }
            state.config = config;
        });
    });
}

fn set_font_input(config: Config) {
    FONT_INPUT.with(|b| {
        plugin_config::set_input(
            b,
            config
                .font_file
                .as_ref()
                .map_or("default", FontFile::filename),
        )
    });
}

// 游戏内存读取、本地玩家识别和坐标投影。
unsafe fn read_u32(address: u32) -> u32 {
    unsafe { ptr::read_volatile(address as *const u32) }
}

unsafe fn read_u8(address: u32) -> u8 {
    unsafe { ptr::read_volatile(address as *const u8) }
}

fn thread_key() -> u32 {
    let value: u32;
    // AArch32 TPIDRURO：只用 TLS 地址区分调用线程，不解引用 OS 内部结构。
    unsafe {
        asm!("mrc p15, 0, {value}, c13, c0, 3", value = out(reg) value, options(nostack, preserves_flags))
    };
    value
}

unsafe fn local_player(base: u32) -> u32 {
    let get: unsafe extern "C" fn(u32) -> u32 = unsafe { mem::transmute(base + 0x00276278) };
    unsafe { get(read_u32(base + 0x01885B58)) }
}

unsafe fn epoch(player: u32) -> Epoch {
    Epoch {
        player,
        room: unsafe { read_u8(player + 0x1054) },
    }
}

unsafe fn display_area(base: u32) -> DisplayArea {
    // 图形管理器 +B0/+B4 同时用于游戏绘图和 nvnWindowSetCrop 的宽高。
    // +00BC0874 将它们传给 +0086A24C；裁剪原点固定为 (0, 0)。
    // 每次投影读取当前值，掌机/主机切换后同步更新插件快照。
    let graphics = unsafe { read_u32(base + 0x02126998) };
    DisplayArea {
        width: unsafe { read_u32(graphics + 0xB0) } as f32,
        height: unsafe { read_u32(graphics + 0xB4) } as f32,
    }
}

unsafe fn damage_is_yellow(base: u32, contact: Contact) -> bool {
    // 在伤害入账时读取已完成设置的攻击字段，不能提前按武器或外观猜瓶爆。
    let flags = unsafe { read_u32(contact.attack + 0x30) };
    if flags & 0x18021 == 0 {
        return true;
    }
    let get_hitzones: unsafe extern "C" fn(u32, u32) -> u32 =
        unsafe { mem::transmute(base + 0x000BAB60) };
    let table = unsafe { get_hitzones(contact.monster, contact.part_index as u32) };
    let hitzones = unsafe { [read_u8(table), read_u8(table + 1), read_u8(table + 2)] };
    let weights = if flags & 0x20 != 0 {
        let player = contact.epoch.player;
        let mut weights = unsafe { [read_u8(player + 0x2B70), read_u8(player + 0x2B71)] };
        // +13CC bit2 的原结算分支只传入本次指定的斩或打权重。
        if unsafe { read_u8(player + 0x13CC) } & 4 != 0 {
            if unsafe { read_u32(contact.attack + 0x40) } & 2 != 0 {
                weights[1] = 0;
            } else {
                weights[0] = 0;
            }
        }
        weights
    } else if flags & 0x10000 != 0 {
        if unsafe { read_u32(contact.attack + 0x40) } & 2 != 0 {
            [100, 0]
        } else {
            [0, 100]
        }
    } else {
        // +30 的 0x8000 分支给斩、打都传入 100；弹分支不使用这两个值。
        [100, 100]
    };
    yellow_for_hitzone(flags, hitzones, weights)
}

impl Projection {
    pub unsafe fn current(base: u32) -> Self {
        let manager = unsafe { read_u32(base + 0x0184D750) };
        let ui = unsafe { read_u32(base + 0x0212668C) };
        let view = unsafe { read_u8(ui + 0xEC) } as u32;
        let record = manager + view * 0x1B0;
        Self {
            function: unsafe { mem::transmute(base + 0x00018A48) },
            manager,
            view,
            width: (unsafe { read_u32(record + 0x70) } as i32
                - unsafe { read_u32(record + 0x68) } as i32) as f32,
            height: (unsafe { read_u32(record + 0x74) } as i32
                - unsafe { read_u32(record + 0x6C) } as i32) as f32,
        }
    }

    pub unsafe fn project(&self, world: [f32; 3]) -> Option<[f32; 2]> {
        let world = Vector([world[0], world[1], world[2], 0.0]);
        let mut projected = Vector([0.0; 4]);
        unsafe {
            (self.function)(
                projected.0.as_mut_ptr(),
                self.manager,
                world.0.as_ptr(),
                self.view,
            )
        };
        normalized_screen(projected.0, self.width, self.height)
    }
}

fn compatible(base: u32) -> bool {
    for (offset, stock, chain_safe) in [
        (0x00170968, [0xE92D41F0, 0xE24DD010], true),
        (0x0009D6F8, [0xE92D4BF0, 0xED2D8B02], true),
        (0x00018A48, [0xE92D4FF0, 0xE28DB01C], false),
        (0x00276278, [0xE92D4010, 0xE1A04000], false),
        (0x000BAB60, [0xE3510008, 0x3A000002], false),
        (0x0086A24C, [0xE92D4800, 0xE24DD008], false),
    ] {
        let actual = unsafe { entry_words(base + offset) };
        if !hook_policy::entry_allowed(actual, stock, chain_safe) {
            error!(
                "HuntLog: unknown instructions at main+0x{offset:08X}; hooks skipped"
            );
            return false;
        }
        if actual != stock {
            info!("HuntLog: chaining Forge hook at main+0x{offset:08X}");
        }
    }
    true
}

unsafe fn entry_words(target: u32) -> [u32; 2] {
    unsafe { [read_u32(target), read_u32(target + 4)] }
}

// 当前加载器的 76 字节 Hook 返回类型在 data.rs 定义。
unsafe extern "C" {
    #[link_name = "forge_hook_create"]
    fn create_hook_raw(
        output: *mut RuntimeHook,
        target: u32,
        detour: u32,
        original: *mut u32,
    ) -> *mut RuntimeHook;
}

// 每次原命中调用绑定栈上下文；嵌套命中结束后恢复上一层。
// 显式清零最大 SRET 缓冲，兼容当前 24 字节与旧版 76 字节返回区。
unsafe fn create_hook(target: u32, detour: u32, original: *mut u32) -> Option<RuntimeHook> {
    let mut hook: RuntimeHook = unsafe { core::mem::zeroed() };
    unsafe {
        ptr::write(original, 0);
        create_hook_raw(&raw mut hook, target, detour, original);
        hook_policy::hook_created(entry_words(target), detour, ptr::read(original)).then_some(hook)
    }
}

static mut PART_RESET_HOOK:MaybeUninit<RuntimeHook>=MaybeUninit::uninit();
static mut PART_RESET_ORIGINAL:u32=0;
unsafe extern "C" fn part_reset(owner:u32,slot:u32,mode:u32){
 use crate::target::Memory;
 let memory=monsters::GameMemory{base:GAME_BASE.load(Ordering::Relaxed)};
 let before_sever=if slot<8{memory.u32(owner+0x1428).and_then(|s|memory.u16(s+0x3c0+slot*12)).map(|v|v as i16)}else{None};
 let original:unsafe extern "C" fn(u32,u32,u32)=unsafe{mem::transmute(PART_RESET_ORIGINAL)};
 unsafe{original(owner,slot,mode)};
 parts::record_reset(&memory,owner,slot,before_sever);
}
static mut REWARDS_HOOK: MaybeUninit<RuntimeHook> = MaybeUninit::uninit();
static mut REWARDS_ORIGINAL:u32=0;
unsafe extern "C" fn reward_screen(object:u32,page:u32)->u32 {
 let base=GAME_BASE.load(Ordering::Relaxed);
 hunt_runtime::reward_snapshot(rewards::capture(&monsters::GameMemory{base},object));
 let original:unsafe extern "C" fn(u32,u32)->u32=unsafe{mem::transmute(REWARDS_ORIGINAL)};
 unsafe{original(object,page)}
}
static mut ITEM_HOOK:MaybeUninit<RuntimeHook>=MaybeUninit::uninit();
static mut ITEM_ORIGINAL:u32=0;
static mut USE_HOOK:MaybeUninit<RuntimeHook>=MaybeUninit::uninit();
static mut USE_ORIGINAL:u32=0;
static mut COLLECT_HOOK:MaybeUninit<RuntimeHook>=MaybeUninit::uninit();
static mut COLLECT_ORIGINAL:u32=0;
static ITEM_CONTEXT:AtomicU32=AtomicU32::new(0);
#[unsafe(naked)]
unsafe extern "C" fn item_bridge(){core::arch::naked_asm!("push {{r4, lr}}","sub sp, sp, #8","str lr, [sp]","bl {handler}","add sp, sp, #8","pop {{r4, pc}}",handler=sym item_mutation);}
#[unsafe(naked)]
unsafe extern "C" fn use_bridge(){core::arch::naked_asm!("push {{r4, lr}}","sub sp, sp, #8","ldr r4, [sp, #16]","str r4, [sp]","str lr, [sp, #4]","bl {handler}","add sp, sp, #8","pop {{r4, pc}}",handler=sym item_use_context);}
#[unsafe(naked)]
unsafe extern "C" fn collect_bridge(){core::arch::naked_asm!("push {{r4, lr}}","sub sp, sp, #8","ldr r4, [sp, #16]","str r4, [sp]","str lr, [sp, #4]","bl {handler}","add sp, sp, #8","pop {{r4, pc}}",handler=sym item_collect_context);}
unsafe extern "C" fn item_use_context(a:u32,b:u32,c:u32,d:u32,e:u32,caller:u32)->u32{
 let old=ITEM_CONTEXT.swap(caller.wrapping_sub(GAME_BASE.load(Ordering::Relaxed)),Ordering::Relaxed);
 let f:unsafe extern "C" fn(u32,u32,u32,u32,u32)->u32=unsafe{mem::transmute(USE_ORIGINAL)};
 let result=unsafe{f(a,b,c,d,e)};ITEM_CONTEXT.store(old,Ordering::Relaxed);result
}
unsafe extern "C" fn item_collect_context(a:u32,b:u32,c:u32,d:u32,e:u32,caller:u32)->u32{
 let old=ITEM_CONTEXT.swap(caller.wrapping_sub(GAME_BASE.load(Ordering::Relaxed)),Ordering::Relaxed);
 let f:unsafe extern "C" fn(u32,u32,u32,u32,u32)->u32=unsafe{mem::transmute(COLLECT_ORIGINAL)};
 let result=unsafe{f(a,b,c,d,e)};ITEM_CONTEXT.store(old,Ordering::Relaxed);result
}
unsafe extern "C" fn item_mutation(pouch:u32,id:u32,delta:u32,flags:u32,caller:u32)->u32{
 use crate::target::Memory;
 let base=GAME_BASE.load(Ordering::Relaxed);let m=monsters::GameMemory{base};
 let local=m.u32(base+0x18584b8)==Some(pouch);
 let before=if local{item_events::count(&m,pouch,id as u16)}else{None};
 let f:unsafe extern "C" fn(u32,u32,u32,u32)->u32=unsafe{mem::transmute(ITEM_ORIGINAL)};
 let result=unsafe{f(pouch,id,delta,flags)};
 if let (Some(before),Some(after))=(before,item_events::count(&m,pouch,id as u16)){
  let change=after-before;
  if change!=0&&change.abs()<=999&&id>0&&id<20000{
   if let Some(slot)=m.u32(base+0x188ca48).and_then(|p|m.u32(p+0x30)).filter(|s|*s<4){
    let caller=caller.wrapping_sub(base);let context=ITEM_CONTEXT.load(Ordering::Relaxed);
    hunt_runtime::item_event(slot+1,id,change,item_events::action(change,caller,context),caller,context);
   }
  }
 }
 result
}

static mut MAP_HOOK:MaybeUninit<RuntimeHook>=MaybeUninit::uninit();
static mut MAP_ORIGINAL:u32=0;
unsafe extern "C" fn map_marker(controller:u32,record:u32,id:u32)->u32{
 use crate::target::Memory;
 let original:unsafe extern "C" fn(u32,u32,u32)->u32=unsafe{mem::transmute(MAP_ORIGINAL)};
 let result=unsafe{original(controller,record,id)};
 let memory=monsters::GameMemory{base:GAME_BASE.load(Ordering::Relaxed)};
 if let Some(node)=map_timers::instance(&memory,controller,id){
  let object=memory.u32(record+0x10).filter(|i|*i<10).and_then(|i|memory.u32(monsters::manager()+0x1c+i*4)).unwrap_or(0);
  MAP_MARKERS.with(|rows|{
   let i=rows.iter().position(|m|m.node==node).or_else(||rows.iter().position(|m|MAP_FRAME.load(Ordering::Relaxed).wrapping_sub(m.frame)>1||m.node==0));
   if let Some(i)=i{rows[i]=map_timers::Marker{node,object:if result!=0{object}else{0},frame:MAP_FRAME.load(Ordering::Relaxed)};}
  });
 }
 result
}

static mut DEBUFF_HOOK:MaybeUninit<RuntimeHook>=MaybeUninit::uninit();
static mut DEBUFF_ORIGINAL:u32=0;
unsafe extern "C" fn debuff_icons(controller:u32){
 let original:unsafe extern "C" fn(u32)=unsafe{mem::transmute(DEBUFF_ORIGINAL)};
 unsafe{original(controller)};
 let memory=monsters::GameMemory{base:GAME_BASE.load(Ordering::Relaxed)};
 let frame=MAP_FRAME.load(Ordering::Relaxed);
 let icons=debuffs::capture(&memory,GAME_BASE.load(Ordering::Relaxed),controller,frame);
 DEBUFF_ICONS.with(|rows|{for icon in icons{if icon.node!=0{let i=rows.iter().position(|r|r.node==icon.node).or_else(||rows.iter().position(|r|r.node==0||frame.wrapping_sub(r.frame)>1));if let Some(i)=i{rows[i]=icon;}}}});
}

struct HookRequest {
    enabled: bool,
    target: u32,
    detour: u32,
    storage: *mut MaybeUninit<RuntimeHook>,
    original: *mut u32,
}

unsafe fn install_hooks(base: u32) -> bool {
    let stock = |offset, words| unsafe { entry_words(base + offset) == words };
    let debuff = [(0x63487c,0xe92d4ff0),(0x634880,0xe24dd03c),(0x29f814,0xe7846000),(0x29f834,0xe7845000),(0x53aa84,0xed911a00)]
        .iter().all(|&(offset, word)| unsafe { read_u32(base + offset) == word });
    macro_rules! request {
        ($enabled:expr, $offset:expr, $detour:ident, $storage:ident, $original:ident) => {
            HookRequest { enabled: $enabled, target: base + $offset,
                detour: $detour as *const () as u32, storage: &raw mut $storage, original: &raw mut $original }
        };
    }
    // Decide every optional feature before changing any entry. Only the two
    // core damage hooks accept chains; caller-sensitive item bridges do not.
    let requests = [
        request!(debuff, 0x63487c, debuff_icons, DEBUFF_HOOK, DEBUFF_ORIGINAL),
        request!(stock(0x61a064, [0xe92d4ff0,0xe28db01c]), 0x61a064, map_marker, MAP_HOOK, MAP_ORIGINAL),
        request!(stock(0x99db8, [0xe92d41f0,0xed2d8b04]), 0x99db8, part_reset, PART_RESET_HOOK, PART_RESET_ORIGINAL),
        request!(stock(0x19b7fc, [0xe92d4ff0,0xe24dd004]), 0x19b7fc, item_bridge, ITEM_HOOK, ITEM_ORIGINAL),
        request!(stock(0x3015e0, [0xe92d41f0,0xe24dd008]), 0x3015e0, use_bridge, USE_HOOK, USE_ORIGINAL),
        request!(stock(0x2e04d4, [0xe92d47f0,0xe24dd008]), 0x2e04d4, collect_bridge, COLLECT_HOOK, COLLECT_ORIGINAL),
        request!(stock(0x386c6c, [0xe92d48f0,0xe24dd018]), 0x386c6c, reward_screen, REWARDS_HOOK, REWARDS_ORIGINAL),
        request!(true, 0x9d6f8, damage, DAMAGE_HOOK, DAMAGE_ORIGINAL),
        request!(true, 0x170968, collision, COLLISION_HOOK, COLLISION_ORIGINAL),
    ];
    let before: [[u32; 2]; 9] = core::array::from_fn(|index| unsafe { entry_words(requests[index].target) });
    for request in &requests {
        if !request.enabled {
            warn!("HuntLog: optional hook main+0x{:08X} signature mismatch; skipped", request.target - base);
        }
    }
    let mut restored = true;
    let installed = hook_policy::install_all(requests.len(), |index| {
        let request = &requests[index];
        if !request.enabled { return true; }
        match unsafe { create_hook(request.target, request.detour, request.original) } {
            Some(hook) => {
                unsafe { ptr::write(request.storage, MaybeUninit::new(hook)); }
                true
            }
            None => {
                error!("HuntLog: hook creation failed at main+0x{:08X}; rolling back", request.target - base);
                false
            }
        }
    }, |index| {
        let request = &requests[index];
        if !request.enabled { return; }
        // Forge permits caller-owned patch buffers. Disabling this patch
        // restores the exact preceding entry, including Damage Numbers' jump,
        // and tells Recomp to restore its function-hook dispatch as well.
        let bytes = before[index].as_ptr().cast::<c_void>();
        let mut patch = forge::sys::patch::Patch { addr: request.target, size: 8,
            patch_bytes: bytes, original_bytes: bytes, enabled: true };
        unsafe { forge::sys::patch::forge_patch_disable(&raw mut patch); }
        if unsafe { entry_words(request.target) } == before[index] {
            unsafe {
                ptr::write(request.original, 0);
                ptr::write(request.storage, MaybeUninit::uninit());
            }
        } else {
            restored = false;
            error!("HuntLog: could not restore hook at main+0x{:08X}", request.target - base);
        }
    });
    if !installed && restored { error!("HuntLog: prior hook entries restored; DPS hooks unavailable"); }
    installed
}

unsafe extern "C" fn collision(
    check: u32,
    attack: u32,
    monster: u32,
    point: u32,
    body: u32,
    attack_point: u32,
    part: u32,
    shape: u32,
) -> u32 {
    let base = GAME_BASE.load(Ordering::Relaxed);
    let mut contact = Contact {
        enabled: false,
        local: false,
        log: false,
        attack,
        monster,
        slot: 0,
        part_index: 0,
        part: 0,
        owner: 0,
        player_slot: 0xFF,
        forwarded: false,
        epoch: Epoch { player: 0, room: 0 },
        world: [0.0; 3],
    };
    if unsafe { read_u8(attack + 0x6D) } == 1 {
        let forwarded = unsafe { read_u8(attack + 0x10) } != 0;
        if !forwarded || unsafe { read_u8(attack + 0x6E) } == 0 {
            let owner = unsafe { read_u32(attack + if forwarded { 0x70 } else { 0x0C }) };
            let player = unsafe { local_player(base) };
            contact.local = owner == player && player >= 0x10000;
            contact.enabled = contact.local;
            contact.log = true;
            contact.owner = owner;
            contact.forwarded = forwarded;
            contact.part = part;
            contact.player_slot = 0xFF;
            let manager = unsafe { read_u32(base + 0x01885B58) };
            if manager >= 0x10000 {
                for i in 0..4u32 {
                    if unsafe { read_u32(manager + 0x1C + i * 4) } == owner {
                        contact.player_slot = i as u8;
                    }
                }
            }
            contact.slot = unsafe { read_u8(part + 8) } as u32;
            contact.part_index = unsafe { read_u8(part + 6) };
            contact.epoch = if player >= 0x10000 {
                unsafe { epoch(player) }
            } else {
                Epoch { player, room: 0 }
            };
            contact.world = unsafe { ptr::read_unaligned(point as *const [f32; 3]) };
            TOTAL_HITS.fetch_add(1, Ordering::Relaxed);
            DEBUG_LOG.with(|log| log.owner_snapshot(owner, player));
        }
    }
    let thread = thread_key();
    let binding = BINDINGS.with(|bindings| {
        let index = bindings
            .0
            .iter()
            .position(|entry| entry.thread == thread)
            .or_else(|| bindings.0.iter().position(|entry| entry.thread == 0))?;
        let previous = bindings.0[index];
        bindings.0[index] = Binding {
            thread,
            contact: &contact,
        };
        Some((index, previous))
    });
    let original: unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32 =
        unsafe { mem::transmute(COLLISION_ORIGINAL) };
    let result = unsafe {
        original(
            check,
            attack,
            monster,
            point,
            body,
            attack_point,
            part,
            shape,
        )
    };
    if let Some((index, previous)) = binding {
        BINDINGS.with(|bindings| bindings.0[index] = previous);
    }
    result
}

unsafe extern "C" fn damage(monster: u32, slot: u32, raw: i32, mode: i32) -> i32 {
    let original: unsafe extern "C" fn(u32, u32, i32, i32) -> i32 =
        unsafe { mem::transmute(DAMAGE_ORIGINAL) };
    let result = unsafe { original(monster, slot, raw, mode) };
    if raw > 0 && result > 0 && mode != 2 && slot < 8 {
        let thread = thread_key();
        let contact = BINDINGS.with(|bindings| {
            bindings
                .0
                .iter()
                .find(|entry| entry.thread == thread)
                .map(|entry| unsafe { *entry.contact })
        });
        if let Some(contact) = contact {
            if contact.log && contact.monster == monster && contact.slot == slot {
                DEBUG_LOG.with(|log| log.hit(&contact, result, raw, mode));
                // Only the verified large-monster snapshot contributes to DPS/history.
                let large=MONSTERS.with(|snapshot|snapshot.entries[..snapshot.count].iter().any(|m|m.object==contact.monster));
                if large {
                    DPS.with(|dps| dps.add(contact.owner, contact.local, result, contact.player_slot));
                    MONSTER_DPS.with(|pool|{
                        if let Some(i)=pool.keys.iter().position(|key|*key==contact.monster).or_else(||pool.keys.iter().position(|key|*key==0)) {
                            pool.keys[i]=contact.monster;
                            pool.states[i].add(contact.owner,contact.local,result,contact.player_slot);
                        }
                    });
                    hunt_runtime::hit(contact.owner,contact.player_slot,contact.monster,result,slot as u8);
                }
                if contact.local {
                    let yellow =
                        unsafe { damage_is_yellow(GAME_BASE.load(Ordering::Relaxed), contact) };
                    STATE.with(|state| {
                        if ENABLED.load(Ordering::Relaxed)
                            && state.push(contact.epoch, contact.world, result, yellow)
                        {
                            TOTAL_EVENTS.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                }
            }
        }
    }
    result
}
