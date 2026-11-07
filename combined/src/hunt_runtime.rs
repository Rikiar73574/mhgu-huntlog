//! Game-thread recorder; disk IO never runs inside damage hooks.
use core::fmt::Write;
use crate::{data::Shared,monsters,plugin_config,quest::{self,Observation},recorder::{self,Recorder,Text,Hit},target::Memory};
struct Queue{hits:[Hit;512],len:usize,dropped:u32}
static HITS:Shared<Queue>=Shared::new(Queue{hits:[Hit::EMPTY;512],len:0,dropped:0});
pub fn hit(owner:u32,slot:u8,monster:u32,damage:i32,part:u8){let player=if slot<4{u32::from(slot)+1}else{owner};HITS.with(|q|{if q.len<512{q.hits[q.len]=Hit{part,player,monster,damage};q.len+=1;}else{q.dropped=q.dropped.saturating_add(1);}});}
struct Runtime{parts:crate::parts::Tracker,rec:Recorder,rewards:crate::rewards::Tracker,status:crate::status_events::Tracker,ready:bool,sample:f32,flush:f32,error:&'static str,stage:u8}
static RUNTIME:Shared<Runtime>=Shared::new(Runtime{parts:crate::parts::Tracker::new(),rec:Recorder::new(),rewards:crate::rewards::Tracker::new(),status:crate::status_events::Tracker::new(),ready:false,sample:0.0,flush:5.0,error:"",stage:0});
// The result UI hook only appends to the recorder buffer; disk IO stays in update.
pub fn reward_snapshot(snapshot:crate::rewards::Snapshot){RUNTIME.with(|r|{
 if r.rec.run==0{return;}
 r.rewards.observe(&snapshot,|group,slot,item|{
  let mut line=Text::<512>::new();
  let _=write!(line,"{{\"schema\":1,\"run\":{},\"event\":\"loot\",\"t\":{},\"source\":\"quest_reward\",\"group\":{},\"slot\":{},\"item_id\":{},\"quantity\":{},\"monster\":null}}",r.rec.run,r.rec.time,group,slot,item.id,item.quantity);
  r.rec.append(line.text());
 });
});}
pub fn status()->(&'static str,u64,u32){RUNTIME.with(|r|(if !r.error.is_empty(){r.error}else if r.rec.run!=0{"Recording hunt"}else{"Waiting for quest"},r.rec.run,r.rec.dropped))}
fn flush(r:&mut Runtime){if r.rec.bytes.len==0{return;}match plugin_config::journal_write(r.rec.offset,&r.rec.bytes.bytes[..r.rec.bytes.len]){Ok(())=>{r.rec.committed();r.error="";},Err(e)=>r.error=e}}
fn identity(m:&monsters::Monster)->recorder::Monster{recorder::Monster{object:m.object,network:m.network,species:m.species}}
pub fn update(base:u32,dt:f32,monsters:&monsters::Snapshot){
 let observation=quest::capture(&monsters::GameMemory{base},base);
 let (hits,len,dropped)=HITS.with(|q|{let out=(q.hits,q.len,q.dropped);q.len=0;q.dropped=0;out});
 RUNTIME.with(|r|{
  r.flush+=dt;
  if !r.ready {
   if r.flush<5.0{return;}r.flush=0.0;
   match plugin_config::journal_size(){Ok(offset)=>{r.rec.initialize(offset);r.ready=true;},Err(e)=>{r.error=e;return;}}
  }
  match observation {
   Observation::Town=>{if r.rec.run!=0{for h in &hits[..len]{r.rec.hit(*h);}r.rec.dropped=r.rec.dropped.saturating_add(dropped);r.rec.end("returned_to_town");crate::parts::reset_maxima();flush(r);}else if r.flush>=5.0{flush(r);r.flush=0.0;}return;},
   Observation::Unknown=>{r.status.begin();r.status.finish();if r.rec.run!=0{for h in &hits[..len]{r.rec.hit(*h);}r.rec.dropped=r.rec.dropped.saturating_add(dropped);}if r.flush>=5.0{flush(r);r.flush=0.0;}return;},
   Observation::Quest{id,elapsed,stage,..}=>{
    if r.rec.run!=0 && r.rec.quest!=id {r.rec.end("quest_changed");flush(r);}
    if r.rec.run==0 {
     if r.rec.bytes.len>recorder::CAP-2048{flush(r);if r.rec.bytes.len>recorder::CAP-2048{return;}}
     r.rec.start(id,elapsed);r.parts=crate::parts::Tracker::new();r.rewards=crate::rewards::Tracker::new();r.status=crate::status_events::Tracker::new();r.sample=1.0;r.stage=stage;
    }
    r.rec.time=r.rec.time.max(elapsed);
   }
  }
  for m in &monsters.entries[..monsters.count]{r.rec.monster_id(identity(m));}
  r.status.begin();
  for m in &monsters.entries[..monsters.count]{if let Some(signals)=m.status{let id=r.rec.monster_id(identity(m));r.status.observe(id,signals,|kind,phase,count|{
   let mut line=Text::<512>::new();let _=write!(line,"{{\"schema\":1,\"run\":{},\"event\":\"status\",\"t\":{},\"monster\":{},\"kind\":\"{}\",\"phase\":\"{}\",\"count\":{},\"source\":\"game_status_transition\"}}",r.rec.run,r.rec.time,id,kind,phase,count);r.rec.append(line.text());
  });}}
  r.status.finish();
  for m in &monsters.entries[..monsters.count]{if let Some(parts)=crate::parts::capture(&monsters::GameMemory{base},m.object,m.species){
   let id=r.rec.monster_id(identity(m));r.parts.observe(id,parts.breaks,|group,new|{
    let mut line=Text::<512>::new();let _=write!(line,"{{\"schema\":1,\"run\":{},\"event\":\"part_break\",\"t\":{},\"monster\":{},\"group\":{},\"phase\":\"{}\"}}",r.rec.run,r.rec.time,id,group,if new{"broken"}else{"observed"});r.rec.append(line.text());
   });
   r.parts.observe_pools(id,&parts,|slot,kind,count|{
    let mut line=Text::<512>::new();let _=write!(line,"{{\"schema\":1,\"run\":{},\"event\":\"part_transition\",\"t\":{},\"monster\":{},\"slot\":{},\"kind\":\"{}\",\"count\":{}}}",r.rec.run,r.rec.time,id,slot,kind,count);r.rec.append(line.text());
   });
  }}
  for h in &hits[..len]{r.rec.hit(*h);}
  r.rec.dropped=r.rec.dropped.saturating_add(dropped);
  r.sample+=dt;
  if r.sample>=1.0{r.sample=0.0;sample(r,base,monsters,observation);}
  if r.flush>=5.0 || r.rec.bytes.len>recorder::CAP/2{flush(r);r.flush=0.0;}
 });
}
fn sample(r:&mut Runtime,base:u32,monsters:&monsters::Snapshot,observation:Observation){
 let mut line=Text::<49152>::new();
 let remaining=if let Observation::Quest{remaining,..}=observation{remaining}else{0};
 let _=write!(line,"{{\"schema\":1,\"run\":{},\"event\":\"sample\",\"t\":{},\"stage\":{},\"quest_remaining_frames\":{},\"dropped_events\":{},\"monsters\":[",r.rec.run,r.rec.time,r.stage,remaining,r.rec.dropped);
 for (i,m) in monsters.entries[..monsters.count].iter().enumerate(){
  if i>0{let _=line.write_char(',');}
  let id=r.rec.monster_id(identity(m));
  let _=write!(line,"{{\"id\":{},\"network_id\":{},\"species\":{},\"hp\":{},\"max_hp\":{},\"area\":{},\"size_scale\":",id,m.network,m.species,m.hp,m.max_hp,m.room);
  if m.size_scale>0.0{let _=write!(line,"{}",m.size_scale);}else{let _=line.write_str("null");}
  crate::parts::append(&mut line,crate::parts::capture(&monsters::GameMemory{base},m.object,m.species));
  let _=line.write_str(",\"name\":");
  let _=recorder::quoted(&mut line,monsters::name(m.species).unwrap_or("Unknown"));
  if let Some(size)=crate::monster_size::capture(&monsters::GameMemory{base},base,monsters::manager(),m.species,m.size_scale){
   let crown=match size.crown{3=>"small_gold",1=>"large_gold",2=>"large_silver",_=>"none"};
   let _=write!(line,",\"size\":{},\"crown\":\"{}\",\"outcome\":null}}",size.value,crown);
  }else{let _=line.write_str(",\"size\":null,\"crown\":null,\"outcome\":null}");}
 }
 let _=line.write_str("],\"players\":[");
 let memory=monsters::GameMemory{base};
 let manager=memory.u32(base.wrapping_add(0x1885b58)).unwrap_or(0);
 let control=memory.u32(base.wrapping_add(0x188ca48)).unwrap_or(0);
 let local_slot=memory.u32(control.wrapping_add(0x30)).filter(|s|*s<4);
 let mut count=0;
 for slot in 0..4u32 {
  let owner=memory.u32(manager.wrapping_add(0x1c+slot*4)).unwrap_or(0);
  if !(0x08000000..0xa8000000).contains(&owner)||owner&3!=0{continue;}
  if count>0{let _=line.write_char(',');}count+=1;
  let name=unsafe{crate::owner_name(base,owner)};
  let _=write!(line,"{{\"id\":{},\"slot\":{},\"name\":",slot+1,slot);
  let _=recorder::quoted(&mut line,core::str::from_utf8(&name.bytes[..name.len as usize]).unwrap_or(""));
  let data=memory.u32(owner+0x13c4).unwrap_or(0);
  let hr=memory.u16(data.wrapping_add(0x554)).filter(|hr|*hr>0 && *hr<=999);
  let _=line.write_str(",\"hr\":");
  if let Some(hr)=hr{let _=write!(line,"{}",hr);}else{let _=line.write_str("null");}
  let _=write!(line,",\"is_local\":{}",local_slot==Some(slot));
  let _=line.write_str(",\"weapon_type\":");
  if let Some(w)=memory.u32(data.wrapping_add(0x14c)).filter(|w|*w<=15){let _=write!(line,"{}",w);}else{let _=line.write_str("null");}
  let _=line.write_str(",\"weapon_id\":");
  if let Some(w)=memory.u32(data.wrapping_add(0x150)){let _=write!(line,"{}",w);}else{let _=line.write_str("null");}
  crate::player_details::append(&mut line,&memory,data);
  let _=line.write_char('}');
 }
 let _=line.write_str("]}");r.rec.append(line.text());
}

// Append only; update performs disk IO. Ignore town and rewards after quest closure.
pub fn item_event(player:u32,id:u32,change:i32,action:&str,caller:u32,context:u32){RUNTIME.with(|r|{
 if r.rec.run==0{return;}
 let mut line=Text::<512>::new();let _=write!(line,"{{\"schema\":1,\"run\":{},\"event\":\"item\",\"t\":{},\"player\":{},\"item_id\":{},\"quantity\":{},\"action\":\"{}\",\"caller\":{},\"context\":{}}}",r.rec.run,r.rec.time,player,id,change,action,caller,context);r.rec.append(line.text());
});}
