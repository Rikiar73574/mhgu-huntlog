#[path="target.rs"] mod target;
#[path="timers.rs"] mod timers;
use target::Memory;
use std::collections::BTreeMap;
struct Mem(BTreeMap<u32,u8>);
impl Mem {fn put(&mut self,a:u32,v:u32){for(i,b)in v.to_le_bytes().iter().enumerate(){self.0.insert(a+i as u32,*b);}}fn float(&mut self,a:u32,v:f32){self.put(a,v.to_bits());}}
impl Memory for Mem{fn u8(&self,a:u32)->Option<u8>{self.0.get(&a).copied()}fn u16(&self,a:u32)->Option<u16>{Some(u16::from_le_bytes([self.u8(a)?,self.u8(a+1)?]))}fn u32(&self,a:u32)->Option<u32>{Some(u32::from_le_bytes([self.u8(a)?,self.u8(a+1)?,self.u8(a+2)?,self.u8(a+3)?]))}}
const P:u32=0x10000000;const E:u32=0x11000000;const S:u32=0x12000000;
#[test]fn drinks_use_remaining_game_time_and_refresh(){let mut m=Mem(BTreeMap::new());m.float(P+0x287c,36000.0);m.float(P+0x2880,18000.0);assert_eq!(timers::drinks(&m,P,Some(0)),[Some(600),Some(300)]);m.float(P+0x287c,60.1);assert_eq!(timers::drinks(&m,P,Some(0))[0],Some(2));m.float(P+0x287c,36000.0);assert_eq!(timers::drinks(&m,P,Some(0))[0],Some(600));}
#[test]fn extreme_temperatures_change_only_affected_drink(){let mut m=Mem(BTreeMap::new());m.float(P+0x287c,36000.0);m.float(P+0x2880,36000.0);assert_eq!(timers::drinks(&m,P,Some(2)),[Some(400),Some(600)]);assert_eq!(timers::drinks(&m,P,Some(4)),[Some(600),Some(400)]);assert_eq!(timers::drinks(&m,P,None),[None,None]);}
#[test]fn expired_and_corrupt_timers_are_hidden(){let mut m=Mem(BTreeMap::new());for v in [0.0,-1.0,f32::NAN,f32::INFINITY,1e9]{m.float(P+0x287c,v);m.float(P+0x2880,v);assert_eq!(timers::drinks(&m,P,Some(0)),[None,None]);}}
#[test]fn paint_tracks_each_actor_independently(){let mut m=Mem(BTreeMap::new());m.put(E+0x1428,S);m.float(S+0x1c4,36000.0);m.put(E+0x10000+0x1428,S+0x10000);m.float(S+0x10000+0x1c4,60.0);assert_eq!(timers::paint(&m,E),Some(600));assert_eq!(timers::paint(&m,E+0x10000),Some(1));m.float(S+0x1c4,0.0);assert_eq!(timers::paint(&m,E),None);assert_eq!(timers::paint(&m,0),None);}
#[test]fn room_temperature_uses_current_room_and_checks_bounds(){let mut m=Mem(BTreeMap::new());m.put(0x200000+0x18977a0,S);m.put(S+0x3bc,S+0x1000);m.put(S+0x1064,2);m.put(S+0x106c,S+0x2000);m.put(P+0x1054,1);m.put(S+0x2000+24+14,4);assert_eq!(timers::temperature(&m,0x200000,P),Some(4));m.put(P+0x1054,2);assert_eq!(timers::temperature(&m,0x200000,P),None);}
#[test]fn local_player_follows_local_slot_not_first_teammate(){let mut m=Mem(BTreeMap::new());let base=0x200000;m.put(base+0x188ca48,S);m.put(S+0x30,1);m.put(base+0x1885b58,E);m.put(E+0x1c,P+0x10000);m.put(E+0x20,P);assert_eq!(timers::local_player(&m,base),Some(P));m.put(S+0x30,4);assert_eq!(timers::local_player(&m,base),None);}
#[test]fn unsupported_code_disables_reader(){let mut m=Mem(BTreeMap::new());assert!(!timers::compatible(&m,0x200000));for(a,v)in [(0x2a6da8,0xed800a00),(0x2a6de8,0xed800a00),(0xc0928,0xe58121c4),(0x29d980,0xeeb70a08),(0x29d9e8,0xeeb70a08),(0x4f9058,0xe0811081),(0x500858,0xe5d0000e)]{m.put(0x200000+a,v);}assert!(timers::compatible(&m,0x200000));m.put(0x200000+0xc0928,0);assert!(!timers::compatible(&m,0x200000));}
#[path="map_timers.rs"]mod map_timers;
#[test]fn map_timer_follows_world_transform_and_hidden_ancestors(){
 let mut m=Mem(BTreeMap::new());let node=P;let root=S;
 m.put(node+0x54,3);m.put(node+0x68,root);m.put(root+0x54,3);m.put(root+0x68,0);
 m.float(node+0x40,1100.0);m.float(node+0x44,120.0);
 assert_eq!(map_timers::position(&m,node),Some((1100.0,120.0)));
 m.float(node+0x40,900.0);assert_eq!(map_timers::position(&m,node),Some((900.0,120.0)));
 m.put(root+0x54,0);assert_eq!(map_timers::position(&m,node),None);
 m.put(root+0x54,3);m.put(root+0x68,node);assert_eq!(map_timers::position(&m,node),None);
 m.put(root+0x68,0);m.float(node+0x40,f32::NAN);assert_eq!(map_timers::position(&m,node),None);
}
#[test]fn map_timer_resolves_both_animation_tables(){
 let mut m=Mem(BTreeMap::new());m.put(P+0x24,S);m.put(S+0x298,20);m.put(S+0xf8,E);m.put(S+0x294,E+0x1000);
 m.put(E+13*4,P+0x1000);m.put(E+0x1000+2*4,P+0x2000);
 assert_eq!(map_timers::instance(&m,P,13),Some(P+0x1000));assert_eq!(map_timers::instance(&m,P,22),Some(P+0x2000));
 assert_eq!(map_timers::instance(&m,0,13),None);assert_eq!(map_timers::instance(&m,P,4096),None);
}
#[path="debuffs.rs"]mod debuffs;
#[test]fn debuff_tracks_selected_icon_and_owner_not_cycle_counter(){
 let mut m=Mem(BTreeMap::new());let base=0x200000;let c=P;let layout=S;let root=P+0x4000;let actor=P+0x8000;
 m.put(c+0x24,layout);m.put(c+0x4c,1);m.put(c+0x50,0);m.put(c+0x54,2);m.put(c+0x58,5);
 m.put(base+0x1885b58,E);m.put(E+0x20,actor);
 m.put(layout+0x298,100);m.put(layout+0xf8,E+0x1000);m.put(E+0x1000+20,root);m.put(root+0xd8,E+0x2000);
 m.put(base+0x18402fc,E+0x3000);m.put(E+0x3000+64+8,7);m.put(E+0x2000+28,P+0xc000);
 m.put(c+0xf0,3);m.put(c+0xf4,20);m.put(c+0xf8,1);
 let t=base+0x1639250+3*16;m.put(t,2);m.put(t+8,0x100);m.put(t+12,0);
 let icons=debuffs::capture(&m,base,c,42);
 assert_eq!(icons[0].actor,actor);assert_eq!(icons[0].node,P+0xc000);assert_eq!(icons[0].low,0x100);assert_eq!(icons[0].frame,42);
 // Real main+18402fc points to relocated main+189fe2c below the heap.
 let static_rows=base+0x189fe2c;m.put(base+0x18402fc,static_rows);m.put(static_rows+64+8,7);
 assert_eq!(debuffs::capture(&m,base,c,42)[0].node,P+0xc000);

 m.put(c+0xf8,0);assert_eq!(debuffs::capture(&m,base,c,43)[0].node,0);
 m.put(c+0xf8,1);m.put(c+0xf0,54);assert_eq!(debuffs::capture(&m,base,c,43)[0].node,0);
}
#[test]fn status_countdown_uses_effect_lifetime_and_reapplication(){
 let mut m=Mem(BTreeMap::new());let icon=debuffs::Icon{node:S,actor:P,group:0,low:0,high:0x07000000,frame:0};
 m.float(P+0x27f0,3600.0);assert_eq!(debuffs::remaining(&m,icon),Some(60));
 m.float(P+0x27f0,60.1);assert_eq!(debuffs::remaining(&m,icon),Some(2));
 m.float(P+0x27f0,3600.0);assert_eq!(debuffs::remaining(&m,icon),Some(60));
 for value in [0.0,-1.0,f32::NAN,f32::INFINITY]{m.float(P+0x27f0,value);assert_eq!(debuffs::remaining(&m,icon),None);}
 assert_eq!(debuffs::remaining(&m,debuffs::Icon{group:4,..icon}),None);
}
#[test]fn drink_icons_use_their_owners_temperature_and_actual_remaining_time(){
 let mut m=Mem(BTreeMap::new());let base=0x200000;
 let cool=debuffs::Icon{node:E,actor:P,group:0,low:0x200,high:0,frame:0};
 let hot=debuffs::Icon{low:0x400,..cool};
 m.float(P+0x287c,36000.0);m.float(P+0x2880,36000.0);
 m.put(base+0x18977a0,S);m.put(S+0x3bc,S+0x1000);m.put(S+0x1064,1);m.put(S+0x106c,S+0x2000);m.put(P+0x1054,0);m.put(S+0x2000+14,2);
 assert_eq!(debuffs::drink_remaining(&m,base,cool),Some(400));assert_eq!(debuffs::drink_remaining(&m,base,hot),Some(600));
 m.put(S+0x2000+14,4);assert_eq!(debuffs::drink_remaining(&m,base,hot),Some(400));
 m.float(P+0x2880,0.0);assert_eq!(debuffs::drink_remaining(&m,base,hot),None);
 assert_eq!(debuffs::drink(debuffs::Icon{low:0x600,..cool}),None);
}

#[test]fn visible_hd_status_icon_does_not_require_override_bit(){
 let mut m=Mem(BTreeMap::new());m.put(P+0x54,0x02000001);m.put(P+0x68,0);m.float(P+0x40,214.0);m.float(P+0x44,114.0);
 assert_eq!(map_timers::position(&m,P),Some((214.0,114.0)));
 m.put(P+0x54,0x02000000);assert_eq!(map_timers::position(&m,P),None);
}

#[test]fn elemental_blight_icons_use_their_own_remaining_timer(){
 let mut m=Mem(BTreeMap::new());
 // Native elemental flags are five pairs (minor/major), not buff flags.
 for (minor,major,offset) in [(1,2,0x28ec),(4,8,0x28f4),(16,32,0x28f8),(64,128,0x28fc),(256,512,0x2900)]{
  m.float(P+offset,1200.0);
  for low in [minor,major]{
   let icon=debuffs::Icon{node:S,actor:P,group:2,low,high:0,frame:0};
   assert_eq!(debuffs::remaining(&m,icon),Some(20),"element flag {low:#x}");
  }
 }
 let thunder=debuffs::Icon{node:S,actor:P,group:2,low:0x20,high:0,frame:0};
 m.float(P+0x28f8,60.1);assert_eq!(debuffs::remaining(&m,thunder),Some(2));
 m.float(P+0x28f8,3600.0);assert_eq!(debuffs::remaining(&m,thunder),Some(60));
 m.float(P+0x28f8,0.0);assert_eq!(debuffs::remaining(&m,thunder),None);
 // Two distinct elemental icons cannot share a truthful countdown.
 assert_eq!(debuffs::remaining(&m,debuffs::Icon{low:1|16,..thunder}),None);
}
