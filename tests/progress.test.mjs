import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';

const code = readFileSync(new URL('../src/lib/presentation.ts', import.meta.url), 'utf8');
// Mock out external imports for pure logic testing
const mocked = code
  .replace(/import\s+.*from\s+["']lucide-svelte["'];?/, 'const Images = {}, Archive = {}, Package = {}, FileText = {};')
  .replace(/import\s+.*from\s+["']\.\/languages["'];?/, 'const getLanguageFlag = () => "";')
  .replace(/import\s+.*from\s+["']\.\/i18n["'];?/, 'const t = (k, params) => params ? `${k}:${JSON.stringify(params)}` : k;');

const js = ts.transpileModule(mocked, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 }
}).outputText;

const { jobProgress, formatJobMessage } = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);

function makeJob(overrides = {}) {
  return {
    id: "test-1",
    status: "running",
    manga: { title: "Test", url: "", cover: "", source_id: "mw", source_name: "MangaWorld", language: "it" },
    volume: {
      title: "Volume 01",
      chapters: [
        { title: "Capitolo 01", url: "https://example.com/1" }
      ]
    },
    chapter: "Capitolo 01",
    chapters_done: 0,
    pages_done: 0,
    pages_total: 20,
    bytes: 1024 * 1024,
    message: "",
    output: "",
    settings: {
      base_url: "",
      output: "",
      delay_ms: 500,
      cbz: false,
      export_format: "pdf",
      sources: []
    },
    ...overrides
  };
}

test('jobProgress returns 100 when job status is completed', () => {
  const job = makeJob({ status: "completed" });
  assert.equal(jobProgress(job), 100);
});

test('jobProgress returns 0 when volume has no chapters', () => {
  const job = makeJob({ volume: { title: "Empty", chapters: [] } });
  assert.equal(jobProgress(job), 0);
});

test('jobProgress reserves export phase headroom for PDF and does not freeze at 99% during download', () => {
  const jobStart = makeJob({ pages_done: 0, pages_total: 20 });
  assert.equal(jobProgress(jobStart), 0);

  const jobMid = makeJob({ pages_done: 10, pages_total: 20 });
  assert.equal(jobProgress(jobMid), 38);

  const jobDownloaded = makeJob({ pages_done: 20, pages_total: 20 });
  // Caps at 74% during download phase so user knows export still has to happen
  assert.equal(jobProgress(jobDownloaded), 74);
});

test('jobProgress advances smoothly during PDF export phase from 75% to 99%', () => {
  const jobExport1 = makeJob({
    phase: "exporting",
    pages_done: 20,
    pages_total: 20,
    export_done: 1,
    export_total: 20,
    message: "Creazione PDF · pagina 1 di 20"
  });
  assert.ok(jobProgress(jobExport1) >= 75 && jobProgress(jobExport1) <= 77);

  const jobExportMid = makeJob({
    phase: "exporting",
    pages_done: 20,
    pages_total: 20,
    export_done: 10,
    export_total: 20,
    message: "Creazione PDF · pagina 10 di 20"
  });
  assert.ok(jobProgress(jobExportMid) >= 86 && jobProgress(jobExportMid) <= 88);

  const jobExportEnd = makeJob({
    phase: "exporting",
    pages_done: 20,
    pages_total: 20,
    export_done: 20,
    export_total: 20,
    message: "Creazione PDF · pagina 20 di 20"
  });
  assert.equal(jobProgress(jobExportEnd), 99);
});

test('jobProgress for images format uses full 0-99% range during download', () => {
  const jobImgMid = makeJob({
    pages_done: 10,
    pages_total: 20,
    settings: {
      base_url: "",
      output: "",
      delay_ms: 500,
      cbz: false,
      export_format: "images",
      sources: []
    }
  });
  assert.equal(jobProgress(jobImgMid), 50);
});

test('formatJobMessage translates known backend messages', () => {
  const waiting = formatJobMessage("In attesa");
  assert.equal(waiting, "downloads.msg.waiting");

  const formatted = formatJobMessage("Creazione PDF · pagina 15 di 20");
  assert.ok(formatted.includes("PDF") && formatted.includes("15") && formatted.includes("20"));

  const already = formatJobMessage("File già esportato · 23 pagine verificate");
  assert.ok(already.includes("23"));
});
