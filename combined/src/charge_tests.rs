//! Guarded, read-only normal-Zinogre charge decoding.
mod target;
#[path="charge.rs"]mod charge;
use std::collections::BTreeMap;
struct Mem(BTreeMap<u32,u32>);
impl target::Memory for Mem{
 fn u8(&self,a:u32)->Option<u8>{self.0.get(&a).map(|v|*v as u8)}
 fn u16(&self,a:u32)->Option<u16>{self.0.get(&a).map(|v|*v as u16)}
 fn u32(&self,a:u32)->Option<u32>{self.0.get(&a).copied()}
}
const B:u32=0x200000;const O:u32=0x08010000;const S:u32=0x08020000;const C:u32=0x08030000;
fn fixture()->Mem{Mem(BTreeMap::from([(O,B+0x17d19bc),(O+0x1428,S),(O+0xcac0,C),(O+0x7420,1),(S+0x5df3,1),(C,100)]))}
#[test]fn normal_charge_and_zero_are_real_values(){let mut m=fixture();assert_eq!(charge::capture(&m,B,O,S,57),Some(charge::Snapshot{active:true,points:Some(100)}));m.0.insert(C,37);assert_eq!(charge::capture(&m,B,O,S,57).unwrap().points,Some(37));m.0.insert(C,0);m.0.insert(S+0x5df3,0);assert_eq!(charge::capture(&m,B,O,S,57),Some(charge::Snapshot{active:false,points:Some(0)}));}
#[test]fn guest_value_is_not_mistaken_for_authoritative_progress(){let mut m=fixture();m.0.insert(O+0x7420,0);assert_eq!(charge::capture(&m,B,O,S,57),Some(charge::Snapshot{active:true,points:None}));}
#[test]fn invalid_meter_keeps_only_verified_state(){let mut m=fixture();for raw in [101,65535]{m.0.insert(C,raw);assert_eq!(charge::capture(&m,B,O,S,57).unwrap().points,None);}m.0.insert(O+0xcac0,1);assert_eq!(charge::capture(&m,B,O,S,57).unwrap().points,None);}
#[test]fn unsupported_species_build_or_state_is_hidden(){let mut m=fixture();assert!(charge::capture(&m,B,O,S,1081).is_none());assert!(charge::capture(&m,B,O,S,60).is_none());m.0.insert(O,B+0x17d19c0);assert!(charge::capture(&m,B,O,S,57).is_none());m=fixture();m.0.insert(S+0x5df3,2);assert!(charge::capture(&m,B,O,S,57).is_none());assert!(charge::capture(&m,B,O,S+4,57).is_none());assert!(charge::capture(&m,B,u32::MAX,S,57).is_none());}
