
// Browser fixture only: no network, no access to the user's persisted queue.
const sources = [{
  id:'example', name:'Example catalog', base_url:'https://example.com', category:'manga',
  language:'en', adapter:'generic', enabled:true, min_delay_ms:500, origin:'created',
  search_path:'/search?keyword={query}&page={page}', result_selector:'a.title',
  chapter_selector:'a.chapter-link', image_selector:'img.scan'
}];
let settings = {base_url:'https://example.com',output:'/tmp/mwr-ui-test',delay_ms:500,cbz:false,export_format:'pdf',sources};
const html = `<html><head><link rel="stylesheet" href="/style.css"></head><body><h1>Pagina campione offline</h1><div class="reading-content"><img class="scan" src="/1.png" onerror="parent.document.body.textContent='UNSAFE'"><img class="scan" data-src="/2.png"></div><a class="chapter" href="/read/2">Capitolo successivo</a><script>parent.document.body.textContent='UNSAFE'<\/script></body></html>`;
const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j3ioAAAAASUVORK5CYII=';
const sample = {id:'12345',source:sources.find(s=>s.id==='example'),url:'https://example.com/read/1',html,created:1,resources:{'https://example.com/1.png':png,'https://example.com/2.png':png,'https://example.com/style.css':`data:text/css;base64,${btoa('h1{color:rgb(120,30,180)}img{width:70px;height:100px;background:url(/1.png)}')}`},warnings:[]};
const chapter = n => ({title:`Capitolo ${n}`,url:`https://example.com/read/${n}`,page_count:20});
const results = Array.from({length:24},(_,i)=>({title:`Titolo ${i+1}`,url:`https://example.com/manga/${i+1}`,cover:'',source_id:'example',source_name:'Example catalog',language:'en'}));
results[0]={...results[0],title:'Invincible',authors:['Other Writer']};
results[1]={...results[1],title:'Invincible (2003)',authors:['Robert Kirkman']};
const searchSample={...structuredClone(sample),id:'10001',url:'https://example.com/search?keyword=invincible&page=1',html:'<article><a class="title" href="/series/1">Invincible</a><span class="author">Robert Kirkman</span></article>'};
const detailSample={...structuredClone(sample),id:'10002',url:'https://example.com/series/1',html:'<h1>Invincible</h1><div class="chapter-list"><a class="chapter-link" href="/read/1">Capitolo 1</a><a class="chapter-link" href="/read/2">Capitolo 2</a></div>'};
const sampleSet=[searchSample,detailSample,sample];
const completedJob={id:'finished',manga:results[1],volume:{title:'Volume scaricato',chapters:[chapter(1)]},status:'completed',chapter:'Capitolo 1',chapters_done:1,pages_done:20,pages_total:20,bytes:100000,message:'Download completato',output:'/tmp/mwr-ui-test/volume.pdf',settings,verification:{state:'verified',verified:20,total:20,repaired:0}};
let jobs=[completedJob,{...structuredClone(completedJob),id:'paused',volume:{title:'Volume in pausa',chapters:[chapter(2)]},status:'paused',message:'In pausa'}];
const events = new Map();
const listeners=new Map();
const emit=(name,payload)=>{for(const [id,event] of listeners)if(event===name)events.get(id)?.({event:name,id,payload:structuredClone(payload)});};
let callbackId = 0;
window.isTauri = true;
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener(){}};
window.__TAURI_INTERNALS__ = {
 transformCallback(fn) {events.set(++callbackId,fn);return callbackId;},
 async invoke(cmd,args) {
  if(cmd === 'plugin:event|listen') {listeners.set(args.handler,args.event);return args.handler;}
  if(cmd === 'plugin:event|unlisten') return;
  if(cmd === 'bootstrap') return {settings:structuredClone(settings),jobs:structuredClone(jobs)};
  if(cmd === 'save_settings') {settings = structuredClone(args.settings);return;}
  if(cmd === 'validate_source') {if(!args.source.name.trim()) throw new Error('Nome obbligatorio');return;}
  if(cmd === 'search_manga') {
    const report={id:'example',name:'Example catalog',count:2,has_next:true,error:null};
    const publish=items=>emit('catalog-results',{request_id:args.requestId,source_id:'example',items,report});
    publish(results.slice(0,2).map(m=>({...m,authors:[]})));
    return new Promise(resolve=>{
      setTimeout(()=>publish(results.slice(0,2)),1500);
      setTimeout(()=>resolve({items:results,has_next:args.page===1,reports:[report]}),12000);
    });
  }
  if(cmd === 'manga_detail') return {manga:args.manga,description:'Dettaglio di prova',volumes:args.manga.url.endsWith('/2')?[{title:'Volume 1',chapters:[chapter(1),chapter(2)]},{title:'Volume 2',chapters:[chapter(3)]}]:[{title:'Capitoli',chapters:[chapter(1),chapter(2),chapter(3)]}]};
  if(cmd === 'chapter_pages') return 20;
  if(cmd === 'enqueue') {
    document.querySelector('#test-output').textContent = JSON.stringify({files:args.volumes.map(v=>({title:v.title,chapters:v.chapters.length}))});return;
  }
  if(cmd === 'lab_samples') return sampleSet.filter(s=>s.source.id===args.sourceId).map(s=>({id:s.id,url:s.url,created:s.created,resources:Object.keys(s.resources).length}));
  if(cmd === 'lab_load') return structuredClone(sampleSet.find(s=>s.id===args.id));
  if(cmd === 'lab_capture') {const value={...structuredClone(sampleSet.find(s=>s.url===args.url)||sample),source:structuredClone(args.source),url:args.url};sampleSet.push(value);return value;}
  if(cmd === 'archive_completed') {jobs=jobs.map(j=>j.status==='completed'?{...j,archived:true}:j);jobs.forEach(j=>emit('job-update',j));return;}
  if(cmd === 'control_job') {const job=jobs.find(j=>j.id===args.id);job.status=args.action==='resume'?'completed':'paused';emit('job-update',job);return;}
  if(cmd === 'remove_job') {jobs=jobs.filter(j=>j.id!==args.id);emit('job-removed',args.id);return;}
  if(cmd === 'open_output') return;
  if(cmd === 'lab_extract') {
    const doc = new DOMParser().parseFromString(args.html,'text/html');
    const selector = args.kind==='reader'?args.source.image_selector:args.kind==='detail'?args.source.chapter_selector:args.source.result_selector;
    return {items:[...doc.querySelectorAll(selector)].map((el,i)=>({title:`Pagina ${i+1}`,url:new URL(el.dataset.src||el.getAttribute('src')||el.getAttribute('href'),args.url).href})),description:'',has_next:false};
  }
  if(cmd === 'lab_download') { document.querySelector('#test-output').textContent=JSON.stringify(args.request);return {output:'/tmp/mwr-ui-test/campione.pdf',pages:2,bytes:1024}; }
  if(cmd === 'plugin:dialog|open') return '/tmp/mwr-ui-test';
  if(cmd === 'lab_cancel' || cmd === 'lab_open_output' || cmd === 'lab_open_browser' || cmd === 'lab_close_browser' || cmd === 'lab_firefox_close' || cmd === 'lab_firefox_inject_picker' || cmd === 'clear_logs' || cmd === 'sources_export_file') return;
  if(cmd === 'lab_firefox_launch') return true;
  if(cmd === 'lab_firefox_status') return { installed: true, running: false, ublock_installed: true };
  if(cmd === 'lab_firefox_get_url' || cmd === 'lab_get_browser_url' || cmd === 'lab_embed_url') return 'https://example.com';
  if(cmd === 'lab_firefox_get_picked') return null;
  if(cmd === 'lab_detect_chapters') return { found: true, selector: 'a.chapter-link', count: 3, sample_titles: ['Capitolo 1', 'Capitolo 2', 'Capitolo 3'] };
  if(cmd === 'get_log_path') return '/tmp/pokko.log';
  if(cmd === 'read_recent_logs') return ['[INFO] Pokko avviato'];
  if(cmd === 'sources_import_file') return structuredClone(sources[0]);
  if(cmd === 'sources_import_folder' || cmd === 'sources_import_github') return structuredClone(sources);
  throw new Error(`Comando non previsto nella fixture: ${cmd}`);
 }
};
await import('../src/main.ts');
const output = document.createElement('output');output.id='test-output';output.style='position:fixed;bottom:0;left:250px;font-size:10px;z-index:500;background:#222;color:white;max-width:600px;max-height:40px;overflow:auto';output.textContent='Fixture locale · backend simulato';document.body.append(output);
