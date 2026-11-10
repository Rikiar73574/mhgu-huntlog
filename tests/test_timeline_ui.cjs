const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const src=fs.readFileSync(require('node:path').join(__dirname,'../history/app.js'),'utf8');
const c={colors:['gold','blue','red'],metric:'hp',monster:'all',showFlinches:true,lo:0,hi:50,esc:String,zoneLabel:(m,s)=>m.parts?.find(p=>p.slot===s)?.name||`Part ${s+1}`,breakLabel:(m,g)=>m.parts?.filter(p=>p.break_groups&(1<<g)).map(p=>p.name).join(' + ')||`Break group ${g+1}`,monsterLabel:(r,m)=>m.name};
vm.createContext(c);
vm.runInContext(src.slice(src.includes('function hpTimeline(')?src.indexOf('function hpTimeline('):src.indexOf('function series('),src.indexOf('function draw(')),c);
vm.runInContext(src.slice(src.indexOf('const eventNames='),src.indexOf('function renderDetail(')),c);
const a={id:1,name:'First',parts:[{slot:0,name:'Head',break_groups:2,requirements_lr:[0,2]}]},b={id:2,name:'Later',parts:[{slot:1,name:'Tail',sever_type:1,sever_required:2}]},future={id:3,name:'Not spawned'};
const r={started_at:100,duration_seconds:50,quest:{rank:'LR'},hits:[],players:[],monsters:[a,b,future],samples:[
 {t:100,monsters:[{id:1,hp:100},{id:3,hp:500,area:255}]},
 {t:110,monsters:[{id:1,hp:0},{id:3,hp:500,area:255}]},
 {t:120,monsters:[{id:1,hp:0},{id:2,hp:200}]},
 {t:130,monsters:[{id:1,hp:0},{id:2,hp:150},{id:3,hp:300,area:1}]},
 {t:140,monsters:[{id:1,hp:0},{id:2,hp:100},{id:3,hp:250,area:1}]},
 {t:150,monsters:[{id:1,hp:0},{id:2,hp:0},{id:3,hp:200,area:1}]}]};
let lines=c.series(r);
assert.equal(lines[0].points.filter(p=>p[1]===0).length,1,'dead monsters must not leave a zero-HP trail');
assert.equal(c.valueAt(lines[0],11),null);assert.equal(c.valueAt(lines[0],10),0);
assert.equal(c.valueAt(lines[1],19),null,'not-yet-spawned monsters stay out of tooltips');
assert.equal(c.valueAt(lines[2],10),null,'allocated actors in area 255 are not spawned');
assert.equal(c.valueAt(lines[1],25),200);
assert.equal(c.plotPoints(lines[0],15,50).length,0);
assert.equal(JSON.stringify(c.plotPoints(lines[0],5,15)),JSON.stringify([[5,100],[10,0]]));
assert.equal(JSON.stringify(c.plotPoints(lines[0],5,9)),JSON.stringify([[5,100],[9,100]]));
c.monster='2';assert.equal(c.series(r)[0].color,'blue','monster colors stay stable when filtered');c.monster='all';
const gap=c.hpTimeline({...r,samples:[{t:100,monsters:[{id:1,hp:100}]},{t:105,monsters:[]},{t:110,monsters:[{id:1,hp:90}]},{t:115,monsters:[{id:1,hp:0}]}]},a);
assert.equal(c.valueAt(gap,6),null,'missing observations must break the HP line');
assert(gap.points.some(p=>p[1]===null));
// Execute the real selector handler: changing a monster must update both zoom and sliders.
const controls=Object.fromEntries(['monster','metric','window','from','to'].map(id=>[id,{}]));
c.$=id=>controls[id];c.draw=()=>{};c.r=r;
vm.runInContext(src.split('\n').find(line=>line.includes("$('monster').onchange=")),c);
function selectMonster(id,run=r){c.r=run;controls.monster.onchange({target:{value:id}});return [c.lo,c.hi];}
assert.deepEqual(selectMonster('2'),[20,50],'selecting a monster fits its spawn/death interval');
assert.deepEqual([Number(controls.from.value),Number(controls.to.value)],[20,50],'range sliders must follow the fitted interval');
assert.deepEqual(selectMonster('1'),[0,10],'dead monsters exclude the rest of the hunt');
assert.deepEqual(selectMonster('3'),[30,50],'unloaded area 255 must not extend the interval');
assert.deepEqual(selectMonster('all'),[0,50],'all monsters restores the full hunt');
assert.deepEqual(selectMonster('2',{...r,hits:[{monster:2,t:118,damage:4}]}),[18,50],'damage before the first HP sample starts the interval');
assert.deepEqual(selectMonster('2',{...r,samples:[],hits:[{monster:2,t:122,damage:4},{monster:2,t:135,damage:8}]}),[22,35],'legacy records fall back to damage times');
assert.deepEqual(selectMonster('2',{...r,samples:[],hits:[]}),[0,50],'missing encounter times retain the full hunt');
assert.deepEqual(selectMonster('1',{...r,hits:[{monster:1,t:140,damage:1}]}),[0,10],'hits after the recorded kill cannot extend the zoom');
assert.deepEqual(selectMonster('2',{...r,samples:[],hits:[{monster:2,t:125,damage:1}]}),[24,26],'a single observation still has a usable zoom window');
selectMonster('all');
c.metric='damage';c.visible=new Set([1]);r.players=[{id:1,name:'Hunter'}];r.hits=[{t:110,player:1,monster:1,damage:10}];
assert.equal(c.valueAt(c.series(r)[0],45),10,'cumulative damage stays visible after the kill');c.metric='hp';
r.part_events=[{t:112,monster:1,slot:0,kind:'flinch',count:2},{t:114,monster:2,slot:1,kind:'secondary',count:1}];
r.part_breaks=[{t:112,monster:1,group:1,phase:'broken'},{t:100,monster:2,group:0,phase:'observed'}];
let events=c.graphEvents(r);
assert.equal(events.filter(e=>e.kind==='break').length,2,'actual break records belong on the timeline');
assert.match(c.eventLabel(events.find(e=>e.kind==='break'&&e.monster===1)),/Head\nBroken/);
assert.match(c.eventLabel(events.find(e=>e.kind==='break'&&e.monster===2)),/already broken/i);
assert.match(c.eventLabel(events.find(e=>e.kind==='secondary')),/Tail\nSever progress 1\/2/);
c.showFlinches=false;events=c.graphEvents(r);assert(!events.some(e=>e.kind==='flinch'));assert(events.some(e=>e.kind==='break'));c.showFlinches=true;
r.part_breaks=[];assert(!c.graphEvents(r).some(e=>e.kind==='break'),'a flinch threshold alone never proves a break');
r.part_events=[{t:114,monster:1,slot:0,kind:'secondary',count:1}];a.parts[0].sever_required=2;
assert.match(c.eventLabel(c.graphEvents(r)[0]),/Additional damage threshold 1\/2/);
assert.doesNotMatch(c.eventLabel(c.graphEvents(r)[0]),/Secondary reaction|Broken/);
assert.notEqual(c.eventGlyph('flinch'),c.eventGlyph('break'));assert.notEqual(c.eventGlyph('sever'),c.eventGlyph('secondary'));
const markers=c.positionEvents([{t:110},{t:110},{t:120}],r,t=>t*2);
assert.notEqual(markers[0].lane,markers[1].lane,'simultaneous event icons must not overlap');
const grouped=c.positionEvents([{t:110,kind:'secondary',count:1},{t:110,kind:'secondary',count:1},{t:110,kind:'break'}],r,t=>t*2);
assert.equal(grouped.length,2);assert.equal(grouped[0].events.length,2);assert.notEqual(grouped[0].lane,grouped[1].lane);
assert.equal(c.positionEvents([{t:110,kind:'break',monster:1},{t:110,kind:'break',monster:2}],r,t=>t*2).length,2);
const tip=c.eventTooltip([{kind:'flinch',partEvent:true,partName:'Head',count:2,flinchTarget:2},{kind:'break',partName:'Head',phase:'broken'}]);
assert.equal(tip.match(/Head/g).length,1);assert.match(tip,/Flinch 2\/2/);assert.match(tip,/Broken/);
console.log('Timeline: encounter auto-zoom, spawn/death bounds, gaps, confirmed breaks and markers passed');
