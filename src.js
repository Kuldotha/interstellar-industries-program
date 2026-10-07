import './style.css';
const Q=16777216n,N=84,R=80,RAW=32,INPUT=116,MAX_NEXT=(1n<<64n)-1n;
const [{recipes,resources,rawResources},wasm]=await Promise.all([fetch('/recipes.json').then(r=>r.json()),WebAssembly.instantiateStreaming(fetch('/engine.wasm'),{})]);
const engine=wasm.instance.exports;
const WORDS=engine.state_words();
const state=()=>new BigUint64Array(engine.memory.buffer,engine.state_ptr(),WORDS);
const rate=v=>Number(v)/Number(Q);
const fmt=v=>v.toLocaleString(undefined,{maximumFractionDigits:3});
const integer=v=>v.toLocaleString();
let busy=false,search='',tier='all',logs=[],sequence=0,supplier=null;
state()[282]=65536n*Q;
engine.recalculate();
document.querySelector('#app').innerHTML=`
<header><div><span class="eyebrow">INTERSTELLAR INDUSTRIES / ENGINE TESTBED</span><h1>Production lab<span class="badge">RUST · WASM</span></h1></div><button id="reset" class="quiet">Reset planet</button></header>
<div class="conditions"><span><i></i> Workforce <b id="workforce">100%</b></span><span><i></i> Health <b>100%</b></span><span><i></i> Power <b>100%</b></span><span>Capacity <b>1 unit / min / building</b></span><label class="workers-control">Available workers <input id="workers" type="number" min="0" max="65536" value="65536" aria-label="Available workers"><button id="apply-workers">Apply</button></label><span id="worker-demand"></span></div>
<section class="summary"><div><label>Buildings</label><strong id="count">0</strong><small>Aggregated by recipe</small></div><div><label>Active recipe groups</label><strong id="groups">0 / 84</strong><small>No per-building solver nodes</small></div><div><label>Next recalculation</label><strong id="boundary">—</strong><small id="reason">Waiting for first build</small></div><div><label>Latest instruction</label><strong id="cu">—</strong><small>Measured SBF · 200,000 CU limit</small></div></section>
<p class="scope">Each building requests 1 worker. Live production supplies consumers first; raw and intermediate stocks cover deficits. The next recalculation is the earliest stock depletion. Storage caps and time advancement are not enabled.</p>
<main><section class="panel catalog"><div class="panel-title"><h2>Build catalogue</h2><span>84 recipes</span></div><div class="filters"><input id="search" placeholder="Search buildings or resources…" aria-label="Search buildings"><select id="tier" aria-label="Filter tier"><option value="all">All tiers</option>${[...new Set(recipes.map(r=>r.tier))].sort((a,b)=>a-b).map(t=>`<option value="${t}">Tier ${t}</option>`).join('')}</select></div><div id="catalogue" class="scroll"></div></section>
<section class="panel production"><div class="panel-title"><h2>Production web</h2><span id="rates-count">0 groups</span></div><div id="production" class="scroll"></div><details><summary>Resource stocks</summary><p>Set stock to cover a production shortage. Drawdown and depletion time appear below.</p><div class="reserve"><select id="resource" aria-label="Resource">${resources.map(r=>`<option>${r}</option>`).join('')}</select><input id="amount" type="number" min="0" max="65536" value="10" step="1" aria-label="Reserve amount"><button id="reserve">Apply stock</button></div><div id="stocks"></div></details></section>
<section class="panel activity"><div class="panel-title"><h2>Action ledger</h2><span id="action-count">0 actions</span></div><p class="ledger-note">Live local SBF measurements. Browser time is reported separately. Instrumentation is included; scheduler overhead is excluded.</p><div id="ledger" class="scroll"><div class="empty">Build a source, then a factory.<br>Each action records its own compute cost.</div></div></section></main>
<footer><span id="status">Engine ready</span><span>Q24 fixed point · shared Rust kernel · no network transactions</span></footer>`;
function drawCatalogue(){
 document.querySelector('#catalogue').innerHTML=recipes.map((r,i)=>({r,i})).filter(({r})=>(!supplier||r.output===supplier)&&(tier==='all'||r.tier===Number(tier))&&[r.building,r.output,...r.inputs].join(' ').toLowerCase().includes(search.toLowerCase())).map(({r,i})=>`<article class="recipe"><div class="recipe-top"><span class="tier">T${r.tier}</span><h3>${r.building}</h3><span class="quantity">×${Number(state()[i]/Q)}</span></div><div class="flow">${r.inputs.length?r.inputs.join(' + '):'<span class="source">Natural source</span>'} <span class="arrow">→</span> <b>${r.output}</b></div><button data-build="${i}" ${busy?'disabled':''}>＋ Build one</button></article>`).join('')||'<div class="empty">No matching recipes</div>';
}
function nextText(next){return next===MAX_NEXT?'Until a change':`${fmt(rate(next))} min`;}
function draw(){
 const s=state(),built=recipes.map((r,i)=>({r,i,count:Number(s[i]/Q),gross:rate(s[INPUT+i])})).filter(x=>x.count);
 document.querySelector('#workforce').textContent=`${fmt(rate(s[283])*100)}%`;
 document.querySelector('#worker-demand').textContent=`Requested: ${fmt(built.reduce((a,x)=>a+x.count,0))} workers`;
 document.querySelector('#count').textContent=integer(built.reduce((a,x)=>a+x.count,0));
 document.querySelector('#groups').textContent=`${built.length} / 84`;
 const next=s[INPUT+N+R];
 document.querySelector('#boundary').textContent=nextText(next);
 document.querySelector('#reason').textContent=next===MAX_NEXT?'Stable until construction or reserve changes':'Earliest stock depletion';
 document.querySelector('#rates-count').textContent=`${built.length} groups`;
 document.querySelector('#production').innerHTML=built.length?`<table><thead><tr><th>Building / output</th><th>Rate / min</th><th>Efficiency</th></tr></thead><tbody>${built.map(({r,count,gross})=>`<tr><td><b>${r.building} <span class="muted">×${count}</span></b><small>${r.output}</small></td><td>${fmt(gross)}</td><td><span class="${gross<0.000001?'blocked':''}">${fmt(gross/count*100)}%</span><div class="meter"><i style="width:${Math.min(100,gross/count*100)}%"></i></div></td></tr>`).join('')}</tbody></table><h3 class="subhead">Net resource flow <span>/ min</span></h3><div class="balances">${resources.map((r,i)=>({r,v:rate(BigInt.asIntN(64,s[INPUT+N+i]))})).filter(x=>Math.abs(x.v)>0.000001).map(({r,v})=>`<div><span>${r}</span><b class="${v<0?'negative':'positive'}">${v>0?'+':''}${fmt(v)}</b></div>`).join('')||'<p class="muted">No net flow</p>'}</div>`:'<div class="empty diagram">○ → ◇ → □<p>Your production web starts here.</p><small>Every building starts at full potential.<br>Missing inputs reduce actual output.</small></div>';
 document.querySelector('#stocks').innerHTML=resources.map((r,i)=>{const amount=s[stockIndex(i)],draw=s[451+i];return amount?`<span>${r}: <b>${fmt(rate(amount))}</b>${draw?` · −${fmt(rate(draw))}/min · ${fmt(rate(amount*Q/draw))} min left`:' · no drawdown'}</span>`:'';}).join('');
 document.querySelectorAll('#reserve,#reset,#apply-workers').forEach(b=>b.disabled=busy);
 drawDemand();
 drawCatalogue();
}
function drawDemand(){
 const s=state(),factor=rate(s[283]);
 const flows=resources.map((resource,r)=>{
  let supply=0,demand=0,used=0;
  recipes.forEach((recipe,i)=>{const gross=rate(s[INPUT+i]);if(recipe.output===resource)supply+=gross;const n=recipe.inputs.filter(x=>x===resource).length;demand+=n*rate(s[i])*factor;used+=n*gross;});
  return {resource,supply,demand,used,gap:Math.max(0,demand-supply)};
 }).filter(x=>x.supply>0||x.demand>0).sort((a,b)=>b.gap-a.gap||a.resource.localeCompare(b.resource));
 if(!flows.length)return;
 document.querySelector('#production').insertAdjacentHTML('afterbegin',`<section class="demand-section"><h3>Resource demand <span>/ min</span></h3><p>Demand is input needed at current workforce capacity. Used is actual consumption; a different missing input can keep it lower.</p><table><thead><tr><th>Resource</th><th>Supply</th><th>Demand</th><th>Used</th></tr></thead><tbody>${flows.map(x=>`<tr class="${x.gap>0.000001?'shortage':''}"><td><b>${x.resource}</b>${x.gap>0.000001?`<small class="negative">Short by ${fmt(x.gap)} / min</small><button data-supplier="${x.resource}">Find producers</button>`:''}</td><td>${fmt(x.supply)}</td><td>${fmt(x.demand)}</td><td>${fmt(x.used)}</td></tr>`).join('')}</tbody></table></section>`);
}
function drawLedger(){
 document.querySelector('#action-count').textContent=`${logs.length} actions`;
 document.querySelector('#ledger').innerHTML=logs.map(x=>`<article class="log ${x.error?'error':''}"><div class="log-title"><span>#${String(x.id).padStart(2,'0')}</span><h3>${x.title}</h3></div><p>${x.detail}</p><dl><dt>Metadata update</dt><dd>${x.cu?`${integer(x.cu.metadataCu)} CU`:'—'}</dd><dt>Rates + next boundary</dt><dd>${x.cu?`${integer(x.cu.solveCu)} CU`:'—'}</dd><dt>Total instruction</dt><dd class="total">${x.cu?`${integer(x.cu.totalCu)} CU`:'—'}</dd></dl>${x.cu?`<div class="budget"><i style="width:${Math.min(100,x.cu.totalCu/2000)}%"></i></div>`:''}<div class="log-bottom"><span>${x.error?'Measurement failed':x.cu?'Wasm / SBF match':'Measuring SBF…'}</span><span>Wasm ${x.ms.toFixed(2)} ms</span></div><small>Next recalculation: ${x.next}</small><details class="group-trace"><summary>${x.groups.filter(g=>g.flag===1).length} recalculated · ${x.groups.filter(g=>g.flag===2).length} direct · ${x.groups.filter(g=>g.flag===3).length} rescaled</summary>${x.groups.map(g=>`<div><span>${g.name}</span><b>${['','Solved','Direct','Rescaled'][g.flag]}</b></div>`).join('')||'<p>All cached rates retained.</p>'}</details>${x.error?`<p class="negative">${escapeText(x.error)}</p>`:''}</article>`).join('');
}
function escapeText(s){const e=document.createElement('span');e.textContent=s;return e.innerHTML;}
async function action(index,title,prepare=()=>{},detail){
 if(busy)return;
 busy=true;const original=Array.from(state());
 prepare();const before=Array.from(state(),String);
 const start=performance.now();let result;
 try{result=index===255?engine.recalculate():engine.build(index);}catch(e){result=3;}
 const ms=performance.now()-start;
 if(result){state().set(original);busy=false;draw();document.querySelector('#status').textContent=`Engine rejected action (${result}); state unchanged.`;return;}
 const expected=Array.from(state(),String);
 const log={id:++sequence,title,detail:detail?`${detail} ${['Full solve','Direct source update','Active dependency update','Exact capacity rescale + active dependencies'][Number(state()[285])]}. Workforce ${fmt(rate(BigInt(before[283]))*100)}% → ${fmt(rate(state()[283])*100)}%.`:`Recipe group capacity: ${fmt(rate(BigInt(before[index])))} → ${fmt(rate(state()[index]))} / min. ${['Full solve','Direct source update','Active dependency update','Exact capacity rescale + active dependencies'][Number(state()[285])]}. Workforce ${fmt(rate(BigInt(before[283]))*100)}% → ${fmt(rate(state()[283])*100)}%. ${Array.from(state().slice(287,371)).filter(x=>x===1n).length} groups recalculated.`,ms,next:nextText(state()[INPUT+N+R])};
 log.groups=Array.from(state().slice(287,371)).map((flag,i)=>({flag:Number(flag),name:recipes[i].building})).filter(x=>x.flag);
 logs.unshift(log);draw();drawLedger();document.querySelector('#status').textContent='Measuring the same action in SBF…';
 try{
  const response=await fetch('/api/measure',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({action:index,state:before})});
  const measured=await response.json();if(!response.ok||measured.error)throw Error(measured.error||'Measurement unavailable');
  if(measured.state.length!==expected.length||measured.state.some((x,i)=>x!==expected[i]))throw Error('Wasm / SBF mismatch; build reverted');
  log.cu=measured;document.querySelector('#cu').textContent=`${integer(measured.totalCu)} CU`;
  document.querySelector('#status').textContent='Verified: Wasm and SBF results match exactly';
 }catch(e){log.error=e.message;state().set(original);document.querySelector('#status').textContent='Action reverted: measurement or verification failed.';}
 busy=false;draw();drawLedger();
}
document.querySelector('#catalogue').addEventListener('click',e=>{const b=e.target.closest('[data-build]');if(b)action(Number(b.dataset.build),`Build ${recipes[b.dataset.build].building}`);});
document.querySelector('#search').addEventListener('input',e=>{search=e.target.value;supplier=null;drawCatalogue();});
document.querySelector('#tier').addEventListener('change',e=>{tier=e.target.value;drawCatalogue();});
function stockIndex(r){const j=rawResources.indexOf(resources[r]);return j<0?371+r:84+j;}
document.querySelector('#reserve').onclick=()=>{const r=resources.indexOf(document.querySelector('#resource').value),amount=Number(document.querySelector('#amount').value);if(!Number.isFinite(amount)||amount<0||amount>65536){document.querySelector('#status').textContent='Reserve must be between zero and 65536.';return;}action(255,'Set '+resources[r]+' stock',()=>{state()[stockIndex(r)]=BigInt(Math.round(amount*Number(Q)));},'Stock edited; CU includes metadata updates and recalculation.');};
document.querySelector('#reset').onclick=()=>action(255,'Reset planet',()=>{state().fill(0n);state()[282]=65536n*Q;document.querySelector('#workers').value='65536';},'Counts and stocks cleared. CU covers recalculation; this test-only reset is not metered.');
draw();

document.querySelector('#apply-workers').onclick=()=>{const workers=Number(document.querySelector('#workers').value);if(!Number.isFinite(workers)||workers<0||workers>65536)return;action(254,'Update available workers',()=>state()[282]=BigInt(Math.round(workers*Number(Q))),`Available workers: ${fmt(workers)}. Capacity changes invalidate only affected components.`);};
document.querySelector('#production').addEventListener('click',e=>{const b=e.target.closest('[data-supplier]');if(!b)return;supplier=b.dataset.supplier;search='';tier='all';document.querySelector('#search').value='';document.querySelector('#search').placeholder=`Producers of ${supplier} · type to clear`;document.querySelector('#tier').value='all';drawCatalogue();});
