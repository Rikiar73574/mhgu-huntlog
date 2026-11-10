//! Read-only MHGU 1.4.0 monster snapshot; all pointer reads stay on update thread.
#[derive(Clone, Copy, Default)]
pub struct Monster { pub object: u32, pub species: u16, pub hp: i32, pub max_hp: i32,
 pub network:u16, pub size_scale:f32, pub room:u8, pub weakness:u8, pub selected:bool, pub revealed:bool, pub rage: f32, pub charge:Option<crate::charge::Snapshot>, pub parts:Option<crate::parts::Snapshot>, pub status:Option<crate::status_events::Signals>, pub ailments: [(i16,i16);7] }
#[derive(Clone, Copy)]
pub struct Snapshot { pub count: usize, pub entries: [Monster;10] }
impl Snapshot { pub const EMPTY: Self=Self { count:0, entries:[Monster {object:0,species:0,hp:0,max_hp:0,network:0,size_scale:0.0,room:0,weakness:0,selected:false,revealed:false,rage:0.0,charge:None,parts:None,status:None,ailments:[(0,0);7]};10] }; }
fn valid(a:u32,n:u32)->bool { a>=0x08000000 && a<0xa8000000 && n<=0xa8000000-a }
unsafe fn read<T:Copy>(a:u32)->T { unsafe { core::ptr::read_volatile(a as *const T) } }
unsafe extern "C" { fn forge_singleton_getInstanceByName(name:*const core::ffi::c_char)->*mut core::ffi::c_void; }
pub(crate) struct Direct;
impl crate::target::Memory for Direct {
 fn u8(&self,a:u32)->Option<u8>{if valid(a,1){Some(unsafe{read(a)})}else{None}}
 fn u16(&self,a:u32)->Option<u16>{if a&1==0 && valid(a,2){Some(unsafe{read(a)})}else{None}}
 fn u32(&self,a:u32)->Option<u32>{if a&3==0 && valid(a,4){Some(unsafe{read(a)})}else{None}}
}
pub fn capture()->Snapshot {
 let mut out=Snapshot::EMPTY;
 let mgr=unsafe { forge_singleton_getInstanceByName(c"sEnemy".as_ptr()) as u32 };
 if !valid(mgr,0x44) || mgr&3!=0 { return out; }
 let base=forge::mem::text_addr();
 let target=crate::target::capture_game(&GameMemory{base},mgr,base);
 for i in 0..10 {
  let obj=unsafe {read::<u32>(mgr+0x1c+4*i)};
  if !valid(obj,0x142c) || obj&3!=0 || out.entries[..out.count].iter().any(|m|m.object==obj) {continue;}
  let s=unsafe {read::<u32>(obj+0x1428)};
  if !valid(s,0x6212) || s&3!=0 {continue;}
  let (id,hp,max_hp,species)=unsafe {(read::<u16>(s+0x1b6),read::<i32>(s+0x370),read::<i32>(s+0x374),read::<u16>(s+0x6210))};
  if id>=10000 || max_hp<=0 || max_hp>200000 || hp<0 || hp>max_hp || name(species).is_none(){continue;}
  let mut m=Monster {object:obj,species,hp,max_hp,selected:target.selected==obj,revealed:target.revealed(obj,species),..Monster::default()};
  m.parts=crate::parts::capture(&GameMemory{base},obj,species);
  if hp>0 {m.charge=crate::charge::capture(&GameMemory{base},base,obj,s,species);}
  m.network=id;
  let size=unsafe {read::<f32>(s+0x1b0)};
  if size.is_finite() && size>0.0 && size<100.0 {m.size_scale=size;}
  m.room=unsafe {read::<u8>(obj+0x1054)};
  if ![12,84,87].contains(&species) {
   let mut best=[0u8;5];
   for part in 0..8 {
    let row=unsafe {read::<u32>(s+0x418+4*part)};
    if !valid(row,10) {continue;}
    let values=unsafe {read::<[u8;10]>(row)};
    crate::weakness::accumulate(&mut best,values);
   }
   m.weakness=crate::weakness::strongest(best);
  }
  if hp>0 {m.status=Some(crate::status_events::Signals{charge:m.charge.map(|c|c.active),flags:unsafe{read::<u8>(s+0x5e08)},counters:unsafe{[read::<u8>(s+0x5d8c),read::<u8>(s+0x5da4),read::<u8>(s+0x5da5),read::<u8>(s+0x5da6)]}});}
  let rage=unsafe {read::<f32>(s+0x514)};
  if rage>=0.0 && rage<=36000.0 {m.rage=rage;}
  m.ailments=crate::status_events::buildups(&Direct,s);
  out.entries[out.count]=m;out.count+=1;
 }
 if let Some(i)=out.entries[..out.count].iter().position(|m|m.selected){out.entries[..=i].rotate_right(1);}
 out
}
pub fn name(id:u16)->Option<&'static str>{ match id {
1=>Some("Rathian"),
2=>Some("Rathalos"),
3=>Some("Khezu"),
4=>Some("Basarios"),
5=>Some("Gravios"),
7=>Some("Diablos"),
8=>Some("Yian Kut-ku"),
9=>Some("Gypceros"),
10=>Some("Plesioth"),
11=>Some("Kirin"),
12=>Some("Lao-Shan Lung"),
13=>Some("Fatalis"),
14=>Some("Velocidrome"),
15=>Some("Gendrome"),
16=>Some("Iodrome"),
17=>Some("Cephadrome"),
18=>Some("Yian Garuga"),
19=>Some("Daimyo Hermitaur"),
20=>Some("Shogun Ceanataur"),
21=>Some("Congalala"),
22=>Some("Blangonga"),
23=>Some("Rajang"),
24=>Some("Kushala Daora"),
25=>Some("Chameleos"),
27=>Some("Teostra"),
30=>Some("Bulldrome"),
32=>Some("Tigrex"),
33=>Some("Akantor"),
34=>Some("Giadrome"),
36=>Some("Lavasioth"),
37=>Some("Nargacuga"),
38=>Some("Ukanlos"),
42=>Some("Barioth"),
43=>Some("Deviljho"),
44=>Some("Barroth"),
45=>Some("Uragaan"),
46=>Some("Lagiacrus"),
47=>Some("Royal Ludroth"),
49=>Some("Agnaktor"),
50=>Some("Alatreon"),
55=>Some("Duramboros"),
56=>Some("Niblesnarf"),
57=>Some("Zinogre"),
58=>Some("Amatsu"),
60=>Some("Arzuros"),
61=>Some("Lagombi"),
62=>Some("Volvidon"),
63=>Some("Brachydios"),
65=>Some("Kecha Wacha"),
66=>Some("Tetsucabra"),
67=>Some("Zamtrios"),
68=>Some("Najarala"),
69=>Some("Seltas Queen"),
70=>Some("Nerscylla"),
71=>Some("Gore Magala"),
72=>Some("Shagaru Magala"),
76=>Some("Seltas"),
77=>Some("Seregios"),
79=>Some("Malfestio"),
80=>Some("Glavenus"),
81=>Some("Astalos"),
82=>Some("Mizutsune"),
83=>Some("Gammoth"),
84=>Some("Nakarkos"),
85=>Some("Great Maccao"),
86=>Some("Valstrax"),
87=>Some("Ahtal-Neset"),
88=>Some("Ahtal-Ka"),
269=>Some("Crimson Fatalis"),
513=>Some("Gold Rathian"),
514=>Some("Silver Rathalos"),
525=>Some("White Fatalis"),
1025=>Some("Dreadqueen Rathian"),
1026=>Some("Dreadking Rathalos"),
1031=>Some("Bloodbath Diablos"),
1042=>Some("Deadeye Yian Garuga"),
1043=>Some("Stonefist Hermitaur"),
1044=>Some("Rustrazor Ceanataur"),
1056=>Some("Grimclaw Tigrex"),
1061=>Some("Silverwind Nargacuga"),
1069=>Some("Crystalbeard Uragaan"),
1081=>Some("Thunderlord Zinogre"),
1084=>Some("Redhelm Arzuros"),
1085=>Some("Snowbaron Lagombi"),
1090=>Some("Drilltusk Tetsucabra"),
1103=>Some("Nightcloak Malfestio"),
1104=>Some("Hellblade Glavenus"),
1105=>Some("Boltreaver Astalos"),
1106=>Some("Soulseer Mizutsune"),
1107=>Some("Elderfrost Gammoth"),
1303=>Some("Furious Rajang"),
1323=>Some("Savage Deviljho"),
1343=>Some("Raging Brachydios"),
1351=>Some("Chaotic Gore Magala"),
_=>None,
}}

// Names-only break reward reference from gatheringhallstudios/MHGenDatabase.
// No live HP or broken-state claims. Tail (sever) is derived from Tail Carve.
pub fn breakable_parts(species: u16) -> Option<&'static str> {
    match species {
        525 => Some("Chest, Eye, Head, Wings"),
        56 => Some("Front Legs, Gills"),
        8 => Some("Beak"),
        1 => Some("Back, Head, Wings, Tail (sever)"),
        2 => Some("Back, Head, Wings, Tail (sever)"),
        3 => Some("Body, Head"),
        4 => Some("Back, Chest, Tail (sever)"),
        5 => Some("Back, Chest, Head, Wings, Tail (sever)"),
        7 => Some("Back, Head, Tail (sever)"),
        9 => Some("Head"),
        10 => Some("Fin, Head, Wings"),
        11 => Some("Head"),
        12 => Some("Back, Head, Horn, Left Shoulder, Right Shoulder"),
        13 => Some("Chest, Eye, Head, Wings"),
        14 => Some("Head"),
        15 => Some("Head"),
        16 => Some("Head"),
        17 => Some("Fin, Head, Tail"),
        18 => Some("Back, Beak, Ears, Wingtalon, Tail (sever)"),
        19 => Some("Claw, Shell"),
        20 => Some("Claws, Shell (Conch), Shell (Gravios), Shell (Wyvern Skull)"),
        21 => Some("Claws, Head"),
        22 => Some("Head, Tail"),
        23 => Some("Head, Tail"),
        24 => Some("Head, Wings, Tail (sever)"),
        25 => Some("Head, Wings, Tail (sever)"),
        27 => Some("Head, Wings, Tail (sever)"),
        32 => Some("Claw, Head, Tail (sever)"),
        33 => Some("Back, Chest, Claws, Head, Tail (sever)"),
        34 => Some("Head"),
        36 => Some("Fin, Head"),
        37 => Some("Head, Tail, Wings, Tail (sever)"),
        38 => Some("Back, Chest, Claws, Head, Tail (sever)"),
        42 => Some("Claws, Head, Tail (sever)"),
        43 => Some("Head, Tail (sever)"),
        44 => Some("Claws, Tail (sever)"),
        45 => Some("Claw, Head, Tail, Tail (sever)"),
        46 => Some("Back, Chest, Claws, Head, Tail (sever)"),
        47 => Some("Head, Sponge, Tail (sever)"),
        49 => Some("Chest, Fin, Head, Legs, Tail (sever)"),
        50 => Some("Head, Legs, Wings, Tail (sever)"),
        55 => Some("Back, Head, Tail (sever)"),
        57 => Some("Back, Claw, Head, Tail (sever)"),
        58 => Some("Back, Head, Left Claw, Right Claw, Tail (sever)"),
        60 => Some("Claws"),
        61 => Some("Head"),
        62 => Some("Back"),
        63 => Some("Head, Pounder, Tail (sever)"),
        65 => Some("Claws, Head, Tail"),
        66 => Some("Back, Back Leg, Jaw"),
        67 => Some("Fin, Head, Leg, Tail"),
        68 => Some("Back, Head, Leg, Tail"),
        69 => Some("Claws, Head, Legs, Tail"),
        70 => Some("Claws, Hide, Spike"),
        71 => Some("Feelers, Head, Wing Arms, Wings, Tail (sever)"),
        72 => Some("Back, Head, Wings, Tail (sever)"),
        76 => Some("Horn"),
        77 => Some("Head, Leg Claws, Wings, Tail (sever)"),
        79 => Some("Back, Head, Tailfeather"),
        80 => Some("Back, Front Leg, Head, Tail, Tail (sever)"),
        81 => Some("Back, Head, Wings, Tail (sever)"),
        82 => Some("Fin, Head, Left Claw, Right Claw, Tail, Tail (sever)"),
        83 => Some("Head, Leg, Trunk"),
        84 => Some("Back, Left Tentacle, Right Tentacle, Shell"),
        85 => Some("Head, Tail"),
        86 => Some("Back, Claws, Head, Wings, Tail (sever)"),
        88 => Some("Back, Belly, Claw, Head"),
        87 => Some("Neset 1, Neset 2, Neset 3, Neset 4, Neset 5"),
        269 => Some("Chest, Eye, Head, Wings"),
        513 => Some("Back, Head, Wings, Tail (sever)"),
        514 => Some("Back, Head, Wings, Tail (sever)"),
        1025 => Some("Back, Head, Wings, Tail (sever)"),
        1026 => Some("Back, Head, Wings, Tail (sever)"),
        1031 => Some("Back, Tail (sever)"),
        1042 => Some("Back, Beak, Ears, Wingtalon, Tail (sever)"),
        1043 => Some("Claw, Shell"),
        1044 => Some("Claws, Shell (Glavenus), Shell (Gravios)"),
        1056 => Some("Claw, Head, Tail (sever)"),
        1061 => Some("Head, Tail, Wings, Tail (sever)"),
        1069 => Some("Claw, Tail, Tail (sever)"),
        1081 => Some("Back, Claw, Head, Tail (sever)"),
        1084 => Some("Claws, Head"),
        1085 => Some("Head"),
        1090 => Some("Back, Back Leg, Jaw"),
        1103 => Some("Back, Head, Tailfeather"),
        1104 => Some("Back, Front Leg, Head, Tail, Tail (sever)"),
        1105 => Some("Back, Head, Wings, Tail (sever)"),
        1106 => Some("Fin, Head, Left Claw, Right Claw, Tail, Tail (sever)"),
        1107 => Some("Head, Leg, Trunk"),
        1303 => Some("Head"),
        1323 => Some("Head, Tail (sever)"),
        1343 => Some("Head, Pounder, Tail (sever)"),
        1351 => Some("Feelers, Head, Wing Arms, Wings, Tail (sever)"),
        _ => None,
    }
}

// Recorder reads verified main-module globals as well as heap objects.
pub(crate) struct GameMemory {pub base:u32}
impl GameMemory {fn allowed(&self,a:u32,n:u32)->bool {valid(a,n) || (a>=self.base && a.checked_add(n).is_some_and(|end| self.base.checked_add(0x2200000).is_some_and(|limit|end<=limit)))}}
impl crate::target::Memory for GameMemory {
 fn u8(&self,a:u32)->Option<u8>{if self.allowed(a,1){Some(unsafe{read(a)})}else{None}}
 fn u16(&self,a:u32)->Option<u16>{if a&1==0&&self.allowed(a,2){Some(unsafe{read(a)})}else{None}}
 fn u32(&self,a:u32)->Option<u32>{if a&3==0&&self.allowed(a,4){Some(unsafe{read(a)})}else{None}}
}
pub(crate) fn manager()->u32 {unsafe{forge_singleton_getInstanceByName(c"sEnemy".as_ptr()) as u32}}
