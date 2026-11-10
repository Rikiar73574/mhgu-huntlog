const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const src=fs.readFileSync(require('node:path').join(__dirname,'../history/app.js'),'utf8');
const c={};vm.createContext(c);vm.runInContext(src.slice(src.indexOf('const eventNames='),src.indexOf('function flinchTarget')),c);
assert.equal(c.eventLabel({kind:'zinogre_charge',phase:'start',count:1}),'Zinogre charged');
assert.equal(c.eventLabel({kind:'zinogre_charge',phase:'end',count:1}),'Zinogre discharged');
assert.match(c.eventLabel({kind:'zinogre_charge',phase:'observed',count:1}),/already charged/);
assert.equal(c.eventLabel({kind:'paralysis',phase:'end',count:1}),'Paralysis ended');
console.log('Charge timeline labels and existing statuses passed');
