import {huntStats,questCategory} from '/home-stats.js';
import {partNames} from '/part-names.js';
import {monsterIcons} from '/monster-icons.js';
import {classicIcons} from '/icon-sets.js';
const $=id=>document.getElementById(id);
const esc=v=>String(v??'Unknown').replace(/ · | — /g,'\n').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const weapons=['Great Sword','Sword & Shield','Hammer','Lance','Heavy Bowgun','Unknown','Light Bowgun','Long Sword','Switch Axe','Gunlance','Bow','Dual Blades','Hunting Horn','Insect Glaive','Charge Blade','Prowler'];
// Same weapon artwork used by the Forge HUD (see combined/LICENSE).
const weaponIcons={"0":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAYElEQVR4nO3RywkAMQgEUNuzP2uwDjvLguBhB0KYze+ywpzEh6gIWW1Qcg0cQffA3mBEvGJmmXMgC6lqZj/4b8bfrBrunkGA/uZyEOEa/AxtAwtAmIZkNYgQnmB6s1nwAT3juTM73f9OAAAAAElFTkSuQmCC","1":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAuUlEQVR4nMXTQQqEIBiGYa/XDTxH4M6r1KJbCC4MISnbeQgP4F6HPviZkAmGAZ0fnkWEbxnG2I+Tcy6Xfd+hlMIurHmwfDnOObit6xy01oJSClJKcJ4nDMMA4zhC/6D3HujjhxCAQsuygBAC+gdp6tA8z6C1hn5B2lqMEehYPIX6B2khDV1P0wTrusL/gsaYcpFSwnEcQMFt26AO3V6kUbDeIuGcAy18CrH3cWsUrG88Tf3ADz9Em+ALOeVypmTCnXgAAAAASUVORK5CYII=","2":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAnUlEQVR4nL2SwQnFIBBEbc+CrEDPHgWbsAIPFiNeLGKDS+ayEEzyN3/gHUKGx7CJMZvUWmlhtKImnHPSIufMjDEYvKcz/xeimFJieu8MArG1ltmeRE2IQgiBQRECGXmC74VSHGNk1JaqCZHWGi2uxPidEAz5XoiCc46RJ8Dz7WVqwiuR/FilFOb1ssfCnUjm52XmqdB7T4tt8W60hQdtcm+/scoaDAAAAABJRU5ErkJggg==","3":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAn0lEQVR4nMXTsQnGIBCGYddzBsHOPdzC3sbGEaxs3cXGyup+7uBrBPlJQHPwFgZ8EhOi1Mvx3hPXe5eISHHqc7CUQtycU8L6OzDGSBwGUM5ZstZK90C87FqrtA6uA8K+8+AYg7iUkrSDcMP1Qc6DWmviVri1JoUQpN3HOw/uYOectDvqPRC/kDFGAoyjY30fBIQN68Z/kDoO7qCncwz8AZq8Xk+9w4JrAAAAAElFTkSuQmCC","4":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAx0lEQVR4nL3SMQqEMBSE4RzN1mt4kOANLK0srKxS2YmCnaVnsLH3APKWDDuwjGtnHPi68GMwzn03jqNFur7vwcnmebaoLEswMxe5x4MaapoGuq4DXV3X8F6QB/M8B16JB7Msg2maYBgG4HlKF7xbCMF+Mey9B/2A94IaaNsW9IrbtsF7QT4Dhhjmz9ErHsdh0bIskC7IkKqqCjTEnedpUfogr3QX1uC+7xat6wqXh/14kNNwURSgQQ25u2fzeJDTB3w98X/Jgh8uMUXu3K5uqQAAAABJRU5ErkJggg==","6":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAk0lEQVR4nM2UywnAIBBEbc+ePOhNPKUSbcCKbGJDBgbCgOSSDRl4h/h5i7ImmFl4k+AmZOacdqFJKQGu43jOGfgLVVRKAb13oAIWiDGC74QU6dG0IEW3dc7C3aVrVMx9/kKNisYYQOdba+A7IduCR+O3Flxr2UWtFfgLuZFoW+jTPOoBtm3zupCCnegp5i787R/7BBPkyha2cJ3WAAAAAElFTkSuQmCC","7":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAh0lEQVR4nM2RsQ3AIAwE2Y0lGIcGQUnPBkzBZkR55RskFwkfJZausGxOtnFuM+YVvXfgfiOkKOcMvPfgeyFFKSUgm2xbSFGtFcgm2xZSVEoBssluC/lQNplcGGOcJ4zWGnh8M7lwXdE6gZW/J2QhhAC4IsVjDMBgbp5iqoX8jLWBOetW3+vCA1Zi13z4Fb55AAAAAElFTkSuQmCC","8":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAjUlEQVR4nL3UwQnAIAwFUNdzCafQPVzEbXSNghNYGvoPDQRDSQz8gyU+m0INQVnrrZTSJ9r9fiCHaq2UUgoFfb339eQcKEHSqOj3B3cQ1jFGCkY/B+4gAEhrjeIP4gEfCeucM4VD15yUMQbFHpQgHMQ/PofUo/4GJYiDW8gNlAo/Oy/1rRKswOetLWMO3tWgAj5HOi8BAAAAAElFTkSuQmCC","9":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAiklEQVR4nMXTwQnAIAyF4aznNIIibuLVHTw7gFul+CBgA9IWSgz8J+lHa5Xo5bCa3jtiZlojc1ADzjm0rB8Cd1CtFZ0HU0o8E6iUgmRyzugcKFBrDcl475Gs24NjDJ7FGJGAIQT0+dj8BuqrJbD+OfoF7MBlk28jn3wO1FdLH5cdZAe6zQNPkBl4AaHexE8//GCGAAAAAElFTkSuQmCC","10":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAl0lEQVR4nL2UwQnAIBAEbU9sSvRjK3ZhNXYhhhzZRxYOEzhdmJc6eHrqJsV9TGtt3iDee8GZC3lglRjjC6zfJ+RgolYqiyE6J0R67/OGRSklAam1CstbNBeyqJQicGl/281OiIk5Z0E79HNCbmAcPou0y9on5EZdPqUnaJf9QiyEiJ+UJuQNjTEEeyHvNIQgrEQIf3/mwgsvK69bYAZAjQAAAABJRU5ErkJggg==","11":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAApElEQVR4nL2RQQoDIRAE/Z6H9SiIFz8i+ChBj/7C5xhsGDCys5uQzA7UybakR6U+nMHMcg7U40K6WEoB1lpwkhMWnlTA1FqBcw5cPCwsTCmNCU3OGRzHAbhVrP8jK9RajwmJjTGAE7XWAN2TF1KQAjFGcCdiP+Vvwt77mOyBpQqGVrHn5IUXgbcVUNU9Jy/kRN57sK/gtrKSFoYQxoSr9nXlX4Uv4SZqu2eouDUAAAAASUVORK5CYII=","12":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAmklEQVR4nMXSMQrEIBRFUbfnliyCG7JyA+IaYukWLCyMlRny4DUfBibjjBFuI/6Dgkp9uVpr51VKCXnvkXocJLTvOzLGIK01eg4klHNG27ah6ZtNg4Q46JxD0ze7DXJQNsZAhNaDIYTzigcI1VqR3LfWonUgDxzHgSTA+NT1oPwOvXdUSkExRkRoPShhOXgbUv8CJfyuj6FfgS95l3g7a6E0mAAAAABJRU5ErkJggg==","13":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAd0lEQVR4nM3TSwrAIAwEUK+XS3iL3M6FN0tRmC4GSvxEqjCLlvISYZrS5CmlWAuea63WgpN+AwGpag9DItJzD7gMHQcBLEPhIF81bLNt8N7N3j44teD3nwPDQABcD++ndzfbBvlqowXmgYLvwsGcs7XM1oQHHgMfY7nogEiCaUgAAAAASUVORK5CYII=","14":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAlklEQVR4nMWSwQmAMAxFu15BcJrO0IOeq7PoFDpAF+gQkQb/JRCqGPXDO5Q0j4TWOSV0ZhwHxj2NmVCKQggM8p+wlEIVTbSuC5NzZr4XbttOFU2k8Z0wxkgVnLGaJvLeMxjgfaHMVVFKiWk+lplQNkogmueJaX54MyFGRmPX9Yx2rzmZmRAFiLCyJmyGrIVolNwWvSU8AJtlXI8us8OqAAAAAElFTkSuQmCC","15":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAi0lEQVR4nL2QPQ6AIAxGOzhwLbkO8TbEicmRBK9Yw1CDtdQ/sMlbmq8PWkBEaAk0F1K5yWHGDAYyV0U5mjsFmgvDEjBTDVQ+QHP9hX72WGJHq8Lz/wlp4Kq4uL/wrqgm7i/kwdwS2lq/n3APScEY44HaAwXtheLKJOADihjUG34RiiunNak8vuFb4Qb1FVvmMinjjAAAAABJRU5ErkJggg=="};
function weaponIcon(type){return classicIcons[type]||weaponIcons[type];}
function quickMetadata(r){
 // started_at is an elapsed game-clock value, never a calendar timestamp.
 const date=typeof r.completed_at==='number'&&r.completed_at>0?new Date(r.completed_at*1000):typeof r.completed_at==='string'?new Date(r.completed_at):null;
 const when=date&&Number.isFinite(date.getTime())?new Intl.DateTimeFormat(undefined,{day:'2-digit',month:'short',year:'numeric',hour:'2-digit',minute:'2-digit'}).format(date):'Date not recorded';
 const recorder=r.players.find(p=>p.id===r.recorder_player_id||p.is_local===true);
 const hr=Number.isInteger(r.recorder_hr)?r.recorder_hr:recorder?.hr;
 return `${when}\nHR ${Number.isInteger(hr)&&hr>0?hr:'—'}`;
}
const colors=['#e7c54c','#71b9db','#e38378','#95bc75','#bf95dc','#e8a96b'];
const crowns={none:'No crown',small_gold:'Small gold',large_gold:'Large gold',large_silver:'Large silver'};
const fmt=n=>Number.isFinite(n)?n.toLocaleString(undefined,{maximumFractionDigits:2}):'Unknown';
const clock=n=>`${Math.floor(n/60)}:${String(Math.floor(n%60)).padStart(2,'0')}`;
const title=r=>r.monsters.map(m=>m.name||`Species ${m.species??'?'}`).join(' / ')||r.quest?.name||'No large monster recorded';
let page=location.hash==='#hunts'?'hunts':'home';
let report=null,selected=null,monster='all',metric='damage',windowSize=10,visible=new Set(),lo=0,hi=0,showFlinches=true;
function syncPage(){
 $('homepage').hidden=page!=='home';$('history-page').hidden=page==='home';
 $('nav-home').setAttribute('aria-current',page==='home'?'page':'false');$('nav-hunts').setAttribute('aria-current',page==='hunts'?'page':'false');
}
function hours(seconds){const h=Math.floor(seconds/3600),m=Math.floor(seconds%3600/60);return h?`${h}h ${m}m`:`${m}m`;}
function renderHome(){
 const runs=report.runs,s=huntStats(runs),recent=runs.slice().reverse().slice(0,4);
 const maxMonsters=Math.max(1,...s.monsterRanks.map(m=>m.encounters)),maxWeapons=Math.max(1,...s.weapons.map(w=>w.count));
 const stat=(label,value,note)=>`<div class="overview-stat"><span>${label}</span><strong>${value}</strong><small>${note}</small></div>`;
 $('homepage').innerHTML=`<div class="overview-heading"><div><p class="eyebrow">YOUR HUNTING JOURNAL</p><h2>Hunting overview.</h2><p class="subtle">Your hunts, monsters and weapons at a glance.</p></div><button id="browse-hunts">Browse hunts →</button></div>
 <div class="overview-stats">${stat('Recorded hunts',fmt(s.runs),`${s.closed} closed\n${s.runs-s.closed} incomplete`)}${stat('Time hunting',hours(s.seconds),'Total recorded time')}${stat('Monsters killed',fmt(s.killed),`${s.captured} captured\n${s.unknown} unknown outcomes`)}${stat('Party damage',fmt(s.damage),'Against large monsters')}${stat('Species encountered',fmt(s.monsters.length),'Across all recorded hunts')}${stat('Crown encounters',fmt(s.crowns),`${s.unknownCrowns} sizes not recorded`)}</div>
 ${!runs.length?'<div class="empty"><h2>Your first hunt starts the story.</h2><p>Recorded hunts will appear here automatically when you refresh.</p></div>':`<div class="overview-columns"><section class="overview-panel"><div class="overview-section-title"><h3>Most hunted monsters</h3><small>Encounters</small></div><div class="overview-rankings">${s.monsterRanks.slice(0,8).map(m=>`<button class="overview-ranking" data-monster="${esc(m.name)}" data-rank="${esc(m.rank)}">${monsterArt(m,'run-monster-icon')}<span><strong>${esc(m.name)} - ${esc(m.rank)}</strong><span class="overview-bar"><i style="width:${m.encounters/maxMonsters*100}%"></i></span><small>${m.killed} killed${m.captured?`\n${m.captured} captured`:''}</small><span class="monster-best-times"><small title="Best recorded duration of a complete single-monster hunt; includes time until the recording ended">${m.best===null?'No best time recorded':`Best time ${clock(m.best)}`}</small></span></span><b>${m.encounters}</b></button>`).join('')}</div></section><section class="overview-panel"><div class="overview-section-title"><h3>Your weapons</h3><small>Hunts used</small></div><div class="overview-rankings">${s.weapons.map(w=>`<button class="overview-ranking" data-weapon-type="${w.type}">${weaponIcon(w.type)?`<img class="weapon-icon" src="${weaponIcon(w.type)}" alt="">`:'<span>?</span>'}<span><strong>${esc(weapons[w.type]||'Unknown weapon')}</strong><span class="overview-bar weapon-bar"><i style="width:${w.count/maxWeapons*100}%"></i></span></span><b>${w.count}</b></button>`).join('')||'<p class="subtle">No local weapon data recorded yet.</p>'}</div>${s.unknownWeapons?`<p class="subtle">Recorder’s weapon unavailable for ${s.unknownWeapons} hunts.</p>`:''}</section></div>
 <section class="overview-recent"><div class="overview-section-title"><h3>Recent hunts</h3><small>Latest saved records</small></div><div class="recent-grid">${recent.map(quickCard).join('')}</div></section>`}
 <p class="subtle">Totals include partial and incomplete recordings. Weapon usage counts the recording hunter once per hunt. ${s.partial?`${s.partial} hunts began recording after departure.`:''}</p>`;
 $('browse-hunts').onclick=()=>showPage('hunts');
 $('homepage').querySelectorAll('[data-run]').forEach(b=>b.onclick=()=>{resetFilters();selectRun(Number(b.dataset.run));});
 $('homepage').querySelectorAll('[data-monster]').forEach(b=>b.onclick=()=>{resetFilters();$('search').value=b.dataset.monster;$('rank').value=b.dataset.rank;showPage('hunts');});
 $('homepage').querySelectorAll('[data-weapon-type]').forEach(b=>b.onclick=()=>{resetFilters();$('weapon').value=b.dataset.weaponType;showPage('hunts');});
}
function resetFilters(){for(const id of ['search','status','crown','weapon','rank'])$(id).value='';}
function showPage(next){page=next;location.hash=next==='home'?'home':'hunts';renderList();}
$('nav-home').onclick=()=>showPage('home');$('nav-hunts').onclick=()=>showPage('hunts');
window.addEventListener('hashchange',()=>{page=location.hash==='#hunts'?'hunts':'home';renderList();});
syncPage();
function options(items,first){return `<option value="">${first}</option>`+items.map(([v,t])=>`<option value="${esc(v)}">${esc(t)}</option>`).join('');}
async function load(){
 $('refresh').disabled=true;
 try{
  const response=await fetch('/api/hunts');const data=await response.json();if(!response.ok)throw Error(data.error);
  for(const r of data.runs)for(const m of r.monsters)if(m.outcome==null&&m.hp===0){m.outcome="killed";m.outcome_source="inferred_zero_hp";}
  report=data;
  const prior=$('weapon').value;
  $('weapon').innerHTML=options([...new Set(data.runs.flatMap(r=>r.players.map(p=>p.weapon_type)).filter(v=>v!=null))].sort((a,b)=>a-b).map(v=>[v,weapons[v]||`Type ${v}`]),'All weapons');$('weapon').value=prior;
  $('notice').textContent=`${data.runs.length} saved runs\n${data.warning_count?`${data.warning_count} journal warnings`:'Journal read successfully'}\nRefreshed ${new Date().toLocaleTimeString()}`;
  renderList();
 }catch(e){$('notice').textContent=e.message;$('detail').innerHTML='<div class="empty"><h2>History unavailable</h2><p>Use Refresh history to try again.</p></div>';}
 finally{$('refresh').disabled=false;}
}
function filtered(){
 const search=$('search').value.toLowerCase().trim(),status=$('status').value,crown=$('crown').value,weapon=$('weapon').value;
 return report.runs.filter(r=>(!$('rank').value||((r.quest?.rank==='G'?'GR':r.quest?.rank)||'Unknown rank')===$('rank').value)&&(!status||(status==='complete'?r.complete:!r.complete))&&(!search||`${title(r)} ${r.quest?.name||''} ${r.quest_id} ${r.players.map(p=>p.name).join(' ')}`.toLowerCase().includes(search))&&(!crown||r.monsters.some(m=>(m.crown??'unknown')===crown))&&(!weapon||r.players.some(p=>String(p.weapon_type)===weapon))).slice().reverse();
}
function quickResult(r){
 if(!r.monsters.length&&questCategory(r))return questCategory(r);
 const outcomes=r.monsters.map(m=>m.outcome??(m.hp===0?'killed':null));
 const killed=outcomes.filter(o=>o==='killed').length,captured=outcomes.filter(o=>o==='captured').length;
 if(outcomes.length&&killed===outcomes.length)return 'Killed';
 if(outcomes.length&&captured===outcomes.length)return 'Captured';
 if(killed||captured)return [killed?`${killed} killed`:null,captured?`${captured} captured`:null,killed+captured<outcomes.length?'others unknown':null].filter(Boolean).join('\n');
 return r.complete?'Outcome unknown':'Incomplete';
}
function quickCard(r){
 const players=r.players.length;
 const cartCount=Number.isInteger(r.carts)?r.carts:'—';
 const cartLimit=Number.isInteger(r.cart_limit)?r.cart_limit:3;
 const gear=r.players.map(p=>{const weapon=weapons[p.weapon_type]||'Unknown weapon',name=p.name||'Unknown hunter',icon=weaponIcon(p.weapon_type);return icon?`<img class="weapon-icon" src="${icon}" alt="${esc(weapon)}" title="${esc(name)}\n${esc(weapon)}">`:`<span class="weapon-unknown" title="${esc(name)}\nUnknown weapon">?</span>`;}).join('');
 return `<button class="run ${r.run===selected?'active':''}" data-run="${r.run}" aria-pressed="${r.run===selected}"><span class="run-top"><span class="run-monster-heading"><span class="run-monster-icons">${r.monsters.map(m=>monsterArt(m,'run-monster-icon')).join('')}</span><strong>${esc(title(r))}</strong></span><span class="duration" aria-label="Time ${clock(r.duration_seconds)}">${clock(r.duration_seconds)}</span></span><span class="run-party"><span>${players} ${players===1?'Player':'Players'}</span><span class="run-weapons">${gear}</span></span><span class="run-bottom"><span class="run-outcome ${quickResult(r)==='Killed'?'killed':''}">${esc(quickResult(r))}</span><span class="run-carts" title="${cartCount==='—'?'Cart count not recorded; default limit shown':'Recorded carts'}">${cartCount}/${cartLimit} Carts</span></span><small class="run-meta">${esc(quickMetadata(r))}</small></button>`;
}
function renderList(){
 if(!report)return;syncPage();if(page==='home'){renderHome();return;}const runs=filtered();$('count').textContent=`${runs.length} ${runs.length===1?'run':'runs'}`;
 $('runs').innerHTML=runs.length?runs.map(quickCard).join(''):'<p class="subtle">No runs match these filters.</p>';
 for(const button of $('runs').querySelectorAll('button'))button.onclick=()=>selectRun(Number(button.dataset.run));
 if(!runs.length){selected=null;$('detail').innerHTML='<div class="empty"><h2>No matching hunts</h2><p>Clear your filters to see all recorded runs.</p></div>';return;}
 if(!runs.some(r=>r.run===selected)){selectRun(runs[0].run);return;}renderDetail();
}
function current(){return report.runs.find(r=>r.run===selected);}
function selectRun(id){page='hunts';location.hash='hunts';selected=id;const r=current();monster=r.monsters.length===1?String(r.monsters[0].id):'all';visible=new Set(r.players.map(p=>p.id));lo=0;hi=r.duration_seconds;renderList();}
function monsterArt(m,cls='monster-art'){const icon=monsterIcons[m.name];return icon?`<img class="${cls}" src="${icon}" alt="" aria-hidden="true">`:'';}
function monsterLabel(r,m){return `${m.name||'Unknown monster'}${r.monsters.filter(x=>x.species===m.species).length>1?`\n${r.monsters.filter(x=>x.species===m.species).findIndex(x=>x.id===m.id)+1}`:''}`;}
function questHeading(r){
 const q=r.quest;
 if(!q)return '<p class="subtle">Quest details unavailable</p>';
 return `<div class="quest-info"><button class="quest-title" aria-describedby="quest-popup">${esc(q.name)} <span aria-hidden="true">ⓘ</span></button><div class="quest-popup" id="quest-popup" role="tooltip"><strong>${esc(q.name)}</strong><p class="quest-rank">${esc(q.hub)} ★${q.stars}\n${esc(q.rank)}\n${esc(q.location)}</p><p><b>Objective</b> ${esc(q.goal)}</p>${q.sub_goal?`<p><b>Subquest</b> ${esc(q.sub_goal)}</p>`:''}<p class="quest-flavor">${esc(q.flavor||'')}</p><div class="quest-facts"><span>${q.time_limit} min limit</span><span>${fmt(q.reward)}z reward</span><span>${fmt(q.fee)}z fee</span>${q.hrp!=null?`<span>${fmt(q.hrp)} HR points</span>`:''}</div></div></div>`;
}
function lootRows(items){return `<ul class="loot-rows">${items.map(x=>`<li><span>${esc(x.name)}</span><strong>×${fmt(x.quantity)}</strong></li>`).join('')}</ul>`;}
function gearDetail(e){
 const ref=e.reference;
 const live=Array.isArray(e.skill_point_names);
 const points=live?e.skill_point_names:ref?.skill_points;
 const lines=[];
 if(points?.length)lines.push(`${live?'':'Base: '}${points.map(x=>`${esc(x.name)} ${x.points>0?'+':''}${esc(x.points)}`).join('\n')}`);
 if(ref)lines.push(`${ref.slots>0?`${esc(ref.slots)} slots\n`:''}Base defense ${esc(ref.base_defense)}`);
 if(ref?.slots!==0&&Array.isArray(e.decoration_names))lines.push(e.decoration_names.length?`Jewels: ${e.decoration_names.map(esc).join('\n')}`:'No jewels equipped');
 else if(ref?.slots!==0)lines.push('Jewels not recorded');
 return `<div class="gear-detail"><span>${esc(e.name)} (Lv ${esc(e.level)})</span>${lines.map(line=>`<small>${line}</small>`).join('')}</div>`;
}
function equipmentDetails(r,p){
 const full=r.players.find(x=>x.id===p.id)||p;
 const rows=values=>`<dl class="loadout-rows">${values.map(([label,value])=>`<div><dt>${esc(label)}</dt><dd>${esc(value??'—')}</dd></div>`).join('')}</dl>`;
 const section=(label,body)=>`<section class="loadout-section"><h4>${label}</h4>${body}</section>`;
 const missing='<p class="subtle">Not recorded</p>';
 const gear=full.equipment?.filter(x=>x&&x.slot>0);
 const stats=full.stats;
 const skillList=names=>names==null?missing:names.length?`<ul class="skill-rows">${names.map(x=>`<li>${esc(x)}</li>`).join('')}</ul>`:'<p class="subtle">None</p>';
 return `<div class="hunter-loadout"><div class="loadout-column">${section('Weapon',full.equipment?.find(x=>x?.slot===0)?gearDetail(full.equipment.find(x=>x?.slot===0)):full.weapon_name?`<p>${esc(full.weapon_name)}</p>`:missing)}${section('Armor / talisman',gear?`<dl class="loadout-rows">${gear.map(x=>`<div><dt>${esc(x.slot_name)}</dt><dd>${gearDetail(x)}</dd></div>`).join('')}</dl>`:missing)}</div><div class="loadout-column">${section('Combat stats',stats?rows([['Attack',stats.attack],['Defense',stats.defense],['Health',stats.health],...['Fire','Water','Thunder','Ice','Dragon'].map((name,i)=>[`${name} resistance`,stats.resistances?.[i]])]):missing)}${section('Equipment skills',skillList(full.skills_names))}${section('Food skills',skillList(full.food_skills_names))}</div></div>`;
}
function huntLoot(r){
 const items=r.reward_items||[];
 const carves=r.players.flatMap(p=>(p.item_groups||[]).filter(g=>g.label==='Possible carves'||g.label==='Carves').map(g=>({...g,player:p.name})));
 const carveBody=carves.length?carves.map(g=>`<div class="loot-group">${r.players.length>1?`<h5>${esc(g.player||'Unknown hunter')}</h5>`:''}${lootRows(g.items)}</div>`).join(''):'<p class="card-note">No carves recorded.</p>';
 const rewardsBody=items.length?`${lootRows(items)}<p class="card-note">Rewards offered at quest completion. Includes items sold or left behind.</p>`:'<p class="card-note">No automatic reward capture recorded for this hunt.</p>';
 return `<div class="hunt-loot-sections"><section class="hunt-loot"><h3>Carves</h3>${carveBody}</section><section class="hunt-loot"><h3>Quest rewards</h3>${rewardsBody}</section></div>`;
}
function zoneLabel(m,slot){return (m.parts||[]).find(p=>p.slot===slot)?.name||partNames[m.species]?.rows.find(p=>p.slot===slot)?.name||`Damage zone ${slot+1}`;}
function breakLabel(m,group){const names=[...new Set((m.parts||[]).filter(p=>p.break_groups&(1<<group)).map(p=>zoneLabel(m,p.slot)))];return names.join(' + ')||`Break group ${group+1}`;}
function countMeter(count,target,label){
 if(!Number.isFinite(count))return '—';
 const known=Number.isFinite(target)&&target>0;
 return `<div class="part-meter"><span>${fmt(count)}${known?`/${fmt(target)}`:''} ${esc(label)}</span>${known?`<meter min="0" max="${target}" value="${Math.max(0,Math.min(count,target))}" aria-label="${esc(label)} ${count} of ${target}"></meter>`:''}</div>`;
}
function partDetails(r,m){
 const hits=r.hits.filter(h=>h.monster===m.id&&Number.isInteger(h.part));
 const pools=m.parts||[];const events=(r.part_breaks||[]).filter(e=>e.monster===m.id);
 const slots=[...new Set([...pools.filter(p=>p.name||p.break_groups||p.sever_required>0||p.flinches>0).map(p=>p.slot),...hits.map(h=>h.part)])].sort((a,b)=>a-b);
 if(!slots.length)return '';
 const rows=slots.map(slot=>{
  const p=pools.find(p=>p.slot===slot),ph=hits.filter(h=>h.part===slot);
  const secondary=p?.sever_required>0&&p.sever_base>0&&p.sever_base<65535;
  const tail=secondary&&p.sever_type===1&&p.name?.includes('Tail');
  const groups=Array.from({length:8},(_,g)=>g).filter(g=>p?.break_groups&(1<<g));
  const broken=groups.filter(g=>(p.broken_groups&(1<<g))||events.some(e=>e.group===g)).length;
  const state=tail?(p.sever_count>=p.sever_required?'Severed':'Severable'):groups.length?(broken===groups.length?'Breakable\nBroken':broken?`Breakable\n${broken}/${groups.length} broken`:'Breakable\nIntact'):secondary?'Breakable':'';
  const target=p?flinchTarget(r,m,{slot,count:p.flinches}):null;
  return `<tr><td>${esc(zoneLabel(m,slot))}${state?`<small>${esc(state)}</small>`:''}</td><td>${p?countMeter(p.flinches,target,'flinches'):'—'}${secondary?countMeter(p.sever_count,p.sever_required<65535?p.sever_required:null,tail?'severs':'secondary reactions'):''}</td><td>${fmt(ph.reduce((n,h)=>n+h.damage,0))}</td><td>${ph.length}</td></tr>`;
 }).join('');
 return `<section class="part-details"><details open><summary>Monster parts</summary><div class="part-table-scroll"><table><thead><tr><th>Part</th><th>Flinch progress</th><th>Damage landed</th><th>Hits</th></tr></thead><tbody>${rows}</tbody></table></div></details></section>`;
}
function monsterDetails(r){return r.monsters.map(m=>`<details class="monster-card" ${r.monsters.length===1?'open':''}><summary><span class="monster-identity">${monsterArt(m)}<span><strong>${esc(monsterLabel(r,m))}</strong><small>${esc(m.outcome==='killed'?'Killed':m.outcome==='captured'?'Captured':'Outcome unknown')}\n${fmt(m.hp)} / ${fmt(m.max_hp)} HP</small></span></span><span class="monster-size">${esc(crowns[m.crown]??'Crown unknown')}<small>Size ${fmt(m.size)}</small></span></summary><div class="monster-hunters">${partDetails(r,m)}<div class="hunter-columns"><span>Hunter / weapon</span><span>Damage</span><span>Hits</span><span>DPS</span></div>${m.players.map(p=>`<details class="hunter-card"><summary><span class="hunter-identity">${weaponIcon(p.weapon_type)?`<img class="weapon-icon" src="${weaponIcon(p.weapon_type)}" alt="${esc(weapons[p.weapon_type]||'Unknown weapon')}">`:''}<span><strong>${esc(p.name||'Unknown hunter')}</strong><small>${esc(weapons[p.weapon_type]||'Unknown weapon')}</small></span></span><span>${fmt(p.damage)}</span><span>${fmt(p.hit_count)}</span><span>${fmt(p.average_dps)}</span></summary>${equipmentDetails(r,p)}</details>`).join('')}</div></details>`).join('');}
const eventNames={sleep:'Sleep',poison:'Poison',paralysis:'Paralysis',stun:'Stun',exhaust_proc:'Exhaust proc',mount_opening:'Mount opening',mount_ended:'Mount ended',blast:'Blast'};
const eventColors={zinogre_charge:'#73b5eb',flinch:'#e7c54c',sever:'#79d4cb',secondary:'#e0a878',sleep:'#8eb1ed',poison:'#c18cdd',paralysis:'#e8ca50',stun:'#e6ac7b',exhaust_proc:'#91b8a2',mount_opening:'#e8e0c8',mount_ended:'#e8e0c8',blast:'#ef8774'};
function eventLabel(e){if(e.kind==='zinogre_charge')return e.phase==='end'?'Zinogre discharged':e.phase==='observed'?'Zinogre already charged when observed':'Zinogre charged';if(e.partEvent)return `${e.partName}\n${e.kind==='flinch'?'Flinch':e.kind==='sever'?'Severed':'Secondary reaction'}${e.kind==='flinch'?` ${e.count}${e.flinchTarget?`/${e.flinchTarget}`:''}`:''}`;return `${eventNames[e.kind]||e.kind}${e.phase==='end'?' ended':e.phase==='observed'?' already active when observed':e.phase==='start'?' started':''}${e.count>1?` ×${e.count}`:''}`;}
function flinchTarget(r,m,e){
 const p=(m.parts||[]).find(p=>p.slot===e.slot);if(!p)return null;
 const rank=r.quest?.rank;const requirements=rank==='LR'?p.requirements_lr:['HR','G','GR'].includes(rank)?p.requirements_hr:null;
 const targets=Array.from({length:8},(_,g)=>g).filter(g=>p.break_groups&(1<<g)).map(g=>requirements?.[g]??(p.requirements_lr?.[g]===p.requirements_hr?.[g]?p.requirements_lr?.[g]:null)).filter(n=>Number.isInteger(n)&&n>0);
 const next=targets.filter(n=>n>=e.count);
 return next.length?Math.min(...next):targets.length?Math.max(...targets):null;
}
function graphEvents(r){
 const parts=(r.part_events||[]).filter(e=>e.kind!=='flinch'||showFlinches).map(e=>{const m=r.monsters.find(m=>m.id===e.monster)||{};return {...e,partEvent:true,partName:zoneLabel(m,e.slot),flinchTarget:e.kind==='flinch'?flinchTarget(r,m,e):null};});
 return [...(r.status_events||[]),...parts].filter(e=>(monster==='all'||String(e.monster)===monster)&&e.t-r.started_at>=lo&&e.t-r.started_at<=hi).sort((a,b)=>a.t-b.t);
}
function renderDetail(){
 const r=current();if(!r)return;
 const damage=r.players.reduce((n,p)=>n+p.damage,0);
 $('detail').innerHTML=`<div class="run-heading"><div><h2>${esc(title(r))}</h2>${questHeading(r)}${r.partial?'<p class="subtle">Recording began after departure</p>':''}</div><span class="badge ${!r.complete||r.partial?'partial':''}">${r.complete?'Returned to town':'Incomplete recording'}</span></div>
 <div class="facts"><div><span>Recorded duration</span><strong>${clock(r.duration_seconds)}</strong></div><div><span>Large-monster damage</span><strong>${fmt(damage)}</strong></div><div><span>Average party DPS</span><strong>${fmt(r.duration_seconds?damage/r.duration_seconds:null)}</strong></div><div><span>Hunters recorded</span><strong>${r.players.length}</strong></div></div>
 <section class="chart-panel" aria-label="Hunt timeline"><div class="chart-controls">${r.monsters.length>1?`<label>Monster<select id="monster"><option value="all">All large monsters</option>${r.monsters.map(m=>`<option value="${m.id}">${esc(monsterLabel(r,m))}</option>`).join('')}</select></label>`:''}<label>Graph<select id="metric"><option value="dps">Damage per second</option><option value="damage">Cumulative damage</option><option value="hp">Monster HP</option></select></label><label>Window<select id="window"><option value="5">5 seconds</option><option value="10">10 seconds</option><option value="30">30 seconds</option><option value="60">60 seconds</option></select></label><label class="timeline-toggle"><input id="flinches" type="checkbox" ${showFlinches?'checked':''}> Flinches</label></div>
 <div class="players">${r.players.map((p,i)=>`<label><input type="checkbox" data-player="${p.id}" ${visible.has(p.id)?'checked':''}><span class="dot" style="background:${colors[i%colors.length]}"></span>${esc(p.name||`Hunter ${p.slot??p.id}`)}</label>`).join('')}</div>
 <div class="plot" id="plot"><svg id="graph" viewBox="0 0 900 310" role="img" aria-label="Hunt timeline graph"></svg><div id="tooltip" class="tooltip"></div></div>
 <p class="chart-note" id="chartnote"></p><div id="status-timeline"></div><div class="range-row"><label>From <span id="from-label"></span><input id="from" type="range" min="0" max="${r.duration_seconds}" step="1" value="${lo}"></label><label>To <span id="to-label"></span><input id="to" type="range" min="0" max="${r.duration_seconds}" step="1" value="${hi}"></label><button id="full" class="quiet">Full hunt</button></div></section>
 <section class="encounters" aria-label="Monster and hunter details">${monsterDetails(r)}</section>${huntLoot(r)}
 <p class="subtle">${r.unattributed_hits.length?`${r.unattributed_hits.length} legacy hits without a verified large-monster target excluded. `:''}Damage is recorded hit damage and can exceed a monster’s starting HP. Killed is inferred when the last recorded HP is zero. Capture requires a separate signal.</p>
 <button id="download" class="download">Download this run as JSON</button>`;
 $('flinches').onchange=e=>{showFlinches=e.target.checked;draw();};
 if($('monster'))$('monster').value=monster;$('metric').value=metric;$('window').value=windowSize;
 if($('monster'))$('monster').onchange=e=>{monster=e.target.value;draw();};$('metric').onchange=e=>{metric=e.target.value;draw();};$('window').onchange=e=>{windowSize=Number(e.target.value);draw();};
 document.querySelectorAll('[data-player]').forEach(el=>el.onchange=()=>{const id=Number(el.dataset.player);el.checked?visible.add(id):visible.delete(id);draw();});
 $('from').oninput=e=>{lo=Math.min(Number(e.target.value),Math.max(0,hi-1));e.target.value=lo;draw();};$('to').oninput=e=>{hi=Math.max(Number(e.target.value),Math.min(r.duration_seconds,lo+1));e.target.value=hi;draw();};
 $('full').onclick=()=>{lo=0;hi=r.duration_seconds;$('from').value=lo;$('to').value=hi;draw();};
 $('download').onclick=()=>{const url=URL.createObjectURL(new Blob([JSON.stringify(r,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download=`hunt-${r.run}.json`;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);};draw();
}
function series(r){
 const hits=r.hits.filter(h=>monster==='all'||String(h.monster)===monster);
 if(metric==='hp')return r.monsters.filter(m=>monster==='all'||String(m.id)===monster).map((m,i)=>({label:monsterLabel(r,m),color:colors[i%colors.length],points:r.samples.flatMap(s=>{const row=s.monsters.find(x=>x.id===m.id);return row&&Number.isFinite(row.hp)?[[s.t-r.started_at,row.hp]]:[];})}));
 return r.players.flatMap((p,i)=>{
  if(!visible.has(p.id))return [];
  const ph=hits.filter(h=>h.player===p.id);let points=[];
  if(metric==='damage'){let total=0;points=[[0,0]];for(const h of ph){const t=h.t-r.started_at;points.push([t,total]);total+=h.damage;points.push([t,total]);}points.push([r.duration_seconds,total]);}
  else{
   const n=Math.max(1,Math.ceil(r.duration_seconds/windowSize)),bins=Array(n).fill(0);
   for(const h of ph)bins[Math.min(n-1,Math.max(0,Math.floor((h.t-r.started_at)/windowSize)))]+=h.damage;
   bins.forEach((d,j)=>{const start=j*windowSize,end=Math.min(r.duration_seconds,start+windowSize),rate=end>start?d/(end-start):0;points.push([start,rate],[end,rate]);});
  }
  return [{label:p.name||`Hunter ${p.id}`,color:colors[i%colors.length],points}];
 });
}
function draw(){
 const r=current(),lines=series(r),svg=$('graph'),x0=58,x1=880,y0=20,y1=270,span=Math.max(hi-lo,.01);
 $('from-label').textContent=clock(lo);$('to-label').textContent=clock(hi);$('window').disabled=metric!=='dps';
 $('chartnote').textContent=metric==='hp'?'HP samples by monster. Player filters apply to damage graphs. Hover or tap the plot to inspect values.':metric==='damage'?'Cumulative damage from departure. Hover or tap the plot to inspect values.':`Damage per second in ${windowSize}-second windows. Small monsters excluded. Hover or tap for values.`;
 const at=(line,t)=>{let last=null;for(const [time,v]of line.points){if(time>t)break;last=v;}return last;};
 const values=lines.flatMap(l=>[at(l,lo),...l.points.filter(p=>p[0]>=lo&&p[0]<=hi).map(p=>p[1])]).filter(v=>v!=null);
 const ymax=Math.max(1,...values)*1.1,x=t=>x0+(t-lo)/span*(x1-x0),y=v=>y1-v/ymax*(y1-y0);
 let markup='<defs><clipPath id="plot-clip"><rect x="58" y="20" width="822" height="250"/></clipPath></defs>';
 for(let i=0;i<=4;i++){const value=ymax*i/4,yy=y(value);markup+=`<line class="grid" x1="${x0}" y1="${yy}" x2="${x1}" y2="${yy}"/><text x="48" y="${yy+4}" text-anchor="end">${fmt(value)}</text>`;}
 for(let i=0;i<=6;i++){const t=lo+span*i/6;markup+=`<text x="${x(t)}" y="297" text-anchor="middle">${clock(t)}</text>`;}
 for(const l of lines){
  // Keep the last observed value at the left edge when zooming.
  const first=at(l,lo),points=[...(first==null?[]:[[lo,first]]),...l.points.filter(p=>p[0]>=lo&&p[0]<=hi)];
  const last=at(l,hi);if(last!=null)points.push([hi,last]);
  const path=points.map(([t,v],i)=>`${i?'L':'M'}${x(t).toFixed(2)},${y(v).toFixed(2)}`).join(' ');
  markup+=`<path class="curve" clip-path="url(#plot-clip)" stroke="${l.color}" d="${path}"/>`;
 }
 if(!lines.length)markup+='<text x="450" y="140" text-anchor="middle">Select a player to display the graph</text>';
 markup+='<line id="cursor" x1="0" y1="20" x2="0" y2="270" stroke="#c4b0a6" stroke-dasharray="3 4" visibility="hidden"/>';
 const events=graphEvents(r);
 for(const e of events){const t=e.t-r.started_at,xx=x(t),label=eventLabel(e),color=eventColors[e.kind]||'#ddd';markup+=`<g class="status-marker"><title>${esc(label)}\n${clock(t)}</title><line x1="${xx}" x2="${xx}" y1="20" y2="270" stroke="${color}" stroke-dasharray="2 5" opacity=".45"/><circle cx="${xx}" cy="${e.phase==='end'?30:20}" r="4" fill="${e.phase==='end'?'#211714':color}" stroke="${color}"/></g>`;}
 $('status-timeline').innerHTML=events.length?`<details class="event-list"><summary>${events.length} timeline events in this view</summary>${events.map(e=>`<div><time>${clock(e.t-r.started_at)}</time><span style="color:${eventColors[e.kind]}">${esc(eventLabel(e))}</span>${monster==='all'?`<small>${esc(monsterLabel(r,r.monsters.find(m=>m.id===e.monster)||{}))}</small>`:''}</div>`).join('')}<p class="chart-note">Observed game-state changes. Mount openings and ended mounts do not confirm a successful topple.</p></details>`:'<p class="chart-note">No events shown in this view.</p>';
 svg.innerHTML=markup;
 svg.onpointermove=e=>{const box=svg.getBoundingClientRect(),px=(e.clientX-box.left)*900/box.width;if(px<x0||px>x1)return hide();const t=lo+(px-x0)/(x1-x0)*span;const cursor=$('cursor');cursor.setAttribute('x1',px);cursor.setAttribute('x2',px);cursor.setAttribute('visibility','visible');const tip=$('tooltip');tip.innerHTML=`<strong>${clock(t)}</strong>`+lines.map(l=>`<div><span style="color:${l.color}">●</span> ${esc(l.label)}: ${fmt(at(l,t))}${metric==='dps'?' /s':''}</div>`).join('');const near=events.filter(ev=>Math.abs(x(ev.t-r.started_at)-px)<8);if(near.length)tip.innerHTML+='<hr>'+near.map(ev=>`<div>${esc(eventLabel(ev)).replace(/\n/g,'<br>')}</div>`).join('');tip.style.display='block';tip.style.left=`${Math.max(0,Math.min(e.clientX-box.left+14,box.width-220))}px`;tip.style.top='15px';};
 function hide(){$('tooltip').style.display='none';$('cursor').setAttribute('visibility','hidden');}svg.onpointerleave=hide;
}
for(const id of ['search','status','crown','weapon','rank'])$(id).addEventListener(id==='search'?'input':'change',renderList);
$('clear').onclick=()=>{for(const id of ['search','status','crown','weapon','rank'])$(id).value='';renderList();};$('refresh').onclick=load;load();

