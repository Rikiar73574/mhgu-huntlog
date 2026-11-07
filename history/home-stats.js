// Aggregate observations without treating a closed recording as a successful quest.
export function huntStats(runs){
 const monsters=new Map(),weapons=new Map();let seconds=0,damage=0,killed=0,captured=0,unknown=0,crowns=0,unknownCrowns=0,unknownWeapons=0;
 for(const r of runs){
  seconds+=Math.max(0,Number.isFinite(r.duration_seconds)?r.duration_seconds:0);
  damage+=(r.players||[]).reduce((n,p)=>n+(Number.isFinite(p.damage)?p.damage:0),0);
  for(const m of r.monsters||[]){const name=m.name||'Unknown monster';const entry=monsters.get(name)||{name,species:m.species,encounters:0,killed:0,captured:0,ranks:{}};entry.encounters++;
   const outcome=m.outcome??(m.hp===0?'killed':null);if(outcome==='killed'){killed++;entry.killed++;}else if(outcome==='captured'){captured++;entry.captured++;}else unknown++;
   const rawRank=r.quest?.rank,rank=rawRank==='G'?'GR':rawRank;
   const rankLabel=['LR','HR','GR'].includes(rank)?rank:'Unknown rank';
   const rankStats=entry.ranks[rankLabel]||(entry.ranks[rankLabel]={encounters:0,killed:0,captured:0,best:null});rankStats.encounters++;if(outcome==='killed')rankStats.killed++;if(outcome==='captured')rankStats.captured++;
   if(r.complete&&!r.partial&&r.monsters.length===1&&['killed','captured'].includes(outcome)&&Number.isFinite(r.duration_seconds)&&r.duration_seconds>0){
    rankStats.best=rankStats.best===null?r.duration_seconds:Math.min(rankStats.best,r.duration_seconds);
   }
   if(['small_gold','large_gold','large_silver'].includes(m.crown))crowns++;else if(!m.crown)unknownCrowns++;
   monsters.set(name,entry);
  }
  const player=(r.players||[]).find(p=>p.is_local===true||p.id===r.recorder_player_id)||(r.players?.length===1?r.players[0]:null);
  if(player&&Number.isInteger(player.weapon_type)){weapons.set(player.weapon_type,(weapons.get(player.weapon_type)||0)+1);}else unknownWeapons++;
 }
 return {runs:runs.length,closed:runs.filter(r=>r.complete).length,partial:runs.filter(r=>r.partial).length,seconds,damage,killed,captured,unknown,crowns,unknownCrowns,unknownWeapons,monsters:[...monsters.values()].sort((a,b)=>b.encounters-a.encounters||a.name.localeCompare(b.name)),monsterRanks:[...monsters.values()].flatMap(m=>Object.entries(m.ranks).map(([rank,stats])=>({name:m.name,species:m.species,rank,...stats}))).sort((a,b)=>b.encounters-a.encounters||a.name.localeCompare(b.name)||a.rank.localeCompare(b.rank)),weapons:[...weapons].map(([type,count])=>({type,count})).sort((a,b)=>b.count-a.count)};
}

// Classify the quest objective, never infer quest type from missing actor data.
export function questCategory(run){
 const goal=run.quest?.goal||'';
 const small=/^Slay \d+ (?:Rhenoplos|Maccao|Jaggi|Jaggia|Velociprey|Genprey|Ioprey|Giaprey|Bullfango|Konchu|Bnahabra|Altaroth|Vespoid|Hornetaur|Remobra|Cephalos|Hermitaur|Ceanataur|Uroktor|Zamite|Ludroth|Conga|Blango|Anteka|Kelbi|Aptonoth|Apceros|Popo|Slagtoth|Larinoth|Gargwa|Moofah)\b/i;
 if(small.test(goal))return 'Small-monster quest';
 if(/^Deliver\b/i.test(goal))return 'Delivery quest';
 return null;
}
