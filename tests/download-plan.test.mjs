import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const js = ts.transpileModule(readFileSync(new URL('../src/download-plan.ts', import.meta.url), 'utf8'), {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const { catalogElements, planDownload } = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const chapter = (title, n) => ({title, url:`https://source.test/chapter/${n}`, page_count:20});
const known = {title:'Volume 01',chapters:[chapter('Capitolo 1',1),chapter('Capitolo 2',2)]};
const unknown = {title:'Capitoli senza volume',chapters:[chapter('Capitolo 3',3),chapter('Capitolo 4',4)]};

test('unknown volume groups become individual selectable chapters without inventing volumes', () => {
  const rows = catalogElements([known, unknown]);
  assert.equal(rows.length,3);
  assert.equal(rows[0],known);
  assert.deepEqual(rows.slice(1).map(v => [v.title,v.chapters.length]),[['Capitolo 3',1],['Capitolo 4',1]]);
  const files = planDownload(rows,'items');
  assert.deepEqual(files.map(v => v.chapters.length),[2,1,1]);
});
test('duplicate source labels remain independently selectable', () => {
  const rows = catalogElements([{title:'Capitoli',chapters:[chapter('Extra',1),chapter('Extra',2)]}]);
  assert.equal(new Set(rows.map(v => v.title)).size,2);
  assert.deepEqual(rows.map(v => v.chapters[0].url),['https://source.test/chapter/1','https://source.test/chapter/2']);
});
test('split and combined exports preserve chapter order, counts and source metadata', () => {
  const split = planDownload([known,unknown],'chapters');
  assert.equal(split.length,4);
  assert.ok(split.every(v => v.chapters.length === 1));
  const combined = planDownload([known,unknown],'combined');
  assert.equal(combined.length,1);
  assert.deepEqual(combined[0].chapters,[...known.chapters,...unknown.chapters]);
  assert.equal(combined[0].chapters[0].page_count,20);
});
test('different selections with the same endpoints do not share queue names', () => {
  const rows = catalogElements([{title:'Capitoli',chapters:[chapter('A',1),chapter('B',2),chapter('C',3)]}]);
  assert.notEqual(planDownload(rows,'combined')[0].title,planDownload([rows[0],rows[2]],'combined')[0].title);
  assert.deepEqual(planDownload([], 'combined'),[]);
  assert.deepEqual(planDownload([rows[0]], 'combined'),[rows[0]]);
});
