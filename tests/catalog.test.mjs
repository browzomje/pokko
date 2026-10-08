import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
async function module(path){const js=ts.transpileModule(readFileSync(new URL(path,import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;return import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);}
const {rankSearchResults}=await module('../src/lib/search-ranking.ts');
const {inferSearchPath}=await module('../src/lib/source-config.ts');
test('title and partial surname rank above title-only and author-only matches',()=>{
 const unrelated={title:'Invincible',authors:['Other Writer'],language:'it'};
 const match={title:'Invincible (2003)',authors:['Robert Kirkman'],language:'en'};
 const other={title:'Walking Dead',authors:['Robert Kirkman'],language:'it'};
 assert.deepEqual(rankSearchResults([unrelated,other,match],'invincible kirk'),[match,unrelated,other]);
});
test('ranking normalizes accents, preserves ties and does not invent authors',()=>{
 const a={title:'Étoile',language:'en'},b={title:'Étoile',language:'it'};
 assert.deepEqual(rankSearchResults([a,b],'etoile'),[b,a]);
 assert.deepEqual(rankSearchResults([a,b],'unknown writer'),[b,a]);
});
test('recording infers encoded and form-encoded search templates',()=>{
 assert.equal(inferSearchPath('https://source.test/search?q=20th+century+boys&page=2','20th century boys'),'/search?q={query}&page={page}');
 assert.equal(inferSearchPath('https://source.test/search/20th%20century%20boys','20th century boys'),'/search/{query}');
 assert.equal(inferSearchPath('https://source.test/manga/1','invincible'),null);
 assert.equal(inferSearchPath('broken','invincible'),null);
});
test('a new connector contains no guessed search path or selectors',async()=>{
 const {newSource,stageFields,stageSignature}=await module('../src/lib/source-config.ts');
 const source=newSource();assert.equal(source.search_path,'');assert.equal(source.base_url,'');assert.equal(source.result_selector,'');assert.equal(source.chapter_selector,'');assert.equal(source.image_selector,'');assert.equal(source.enabled,false);
 assert.deepEqual(stageFields.search,['result_selector','next_selector']);assert.ok(!stageFields.reader.includes('chapter_selector'));
 const before=stageSignature(source,'search','html','https://source.test/search');source.image_selector='img.scan';assert.equal(stageSignature(source,'search','html','https://source.test/search'),before);source.result_selector='a.title';assert.notEqual(stageSignature(source,'search','html','https://source.test/search'),before);
});
test('a template replaces the observed query value, not a coincidental substring',()=>{
 assert.equal(inferSearchPath('https://source.test/search?query=A','a'),'/search?query={query}');
 assert.equal(inferSearchPath('https://source.test/search','a'),null);
 assert.equal(inferSearchPath('https://source.test/search?q=Invincible','invincible'),'/search?q={query}');
 assert.equal(inferSearchPath('https://source.test/#/search?q=invincible','invincible'),null);
});

test('dynamic creator ranking works with unrelated titles and incoming metadata',async()=>{
 const {mergeSourceResults}=await module('../src/lib/search-progress.ts');
 const a={title:'Saga',url:'/saga',source_id:'a',authors:[],language:'en'},b={title:'Saga of the North',url:'/other',source_id:'b',language:'en'};
 let items=mergeSourceResults([b],'a',[a,a]);assert.equal(items.length,2);
 items=mergeSourceResults(items,'a',[{...a,authors:['Brian K. Vaughan']}]);
 assert.equal(items.length,2);assert.equal(rankSearchResults(items,'saga vaughan')[0].url,'/saga');
 assert.deepEqual(mergeSourceResults(items,'b',[]).map(m=>m.url),['/saga']);
});
