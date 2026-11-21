// Извлекает данные (кролик, палитры, тарифы, документы, иконки) из оригинального Luna.html
const fs=require('fs'),vm=require('vm');
const L=fs.readFileSync('/mnt/user-data/uploads/Luna.html','utf8').split('\n');
const r=(a,b)=>L.slice(a-1,b).join('\n');
const code=[r(350,354),r(361,368),r(409,409),r(448,451),r(453,453),r(455,455),r(536,537),r(651,651),r(664,669),r(673,716)].join('\n');
const o=vm.runInNewContext(code+'\n;({RB,P,M,TPL,RB_STOP,RB_LIM,IC,FEAT,PRE,LG,DOCS})');
const out={rb:o.RB,rb_stop:o.RB_STOP,rb_lim:o.RB_LIM,
 pal:o.P.map(p=>({n:p.n,a:p.a,b:p.b,c:p.c})),
 models:o.M.map(m=>({n:m.n,d:m.d,pro:!!m.pro})),
 feat:o.FEAT,pre:o.PRE.map(p=>({n:p.n,who:p.who,rules:p.rules})),
 tpl_stop:o.TPL.stop,tpl_limit:o.TPL.limit,icons:o.IC,lg_date:o.LG.date,
 docs:Object.fromEntries(Object.entries(o.DOCS).map(([k,d])=>[k,{h:d.h,t:d.t,s:d.s}]))};
fs.writeFileSync('frontend/assets/data.json',JSON.stringify(out));
console.log(Object.keys(out.docs),out.rb.length,out.pal.length,Object.keys(out.icons).length);
