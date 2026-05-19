# MIRA Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a browser-only Vite + TypeScript + Plotly.js dashboard to visualize MIRA (Mutation In RNA-seq Aligner) results for AR variants in mCRPC patients.

**Architecture:** All 20 sample TSV files are parsed entirely in the browser with no backend. A plain TypeScript state object in `store.ts` drives all UI updates reactively. Charts (Plotly.js) and components (plain HTML/TS) are mounted into a three-column layout: sidebar → heatmap → detail drawer.

**Tech Stack:** Vite 5, TypeScript 5, Plotly.js 2, Vitest (unit tests), plain CSS with custom properties.

---

## File Map

| File | Responsibility |
|------|----------------|
| `index.html` | App shell, layout skeleton, Plotly CDN |
| `vite.config.ts` | Vite config with path aliases |
| `tsconfig.json` | Strict TypeScript config |
| `vitest.config.ts` | Test runner config |
| `src/types.ts` | All shared TypeScript interfaces and types |
| `src/parser.ts` | TSV → `MiraRecord[]` parsing + cohort/therapy inference |
| `src/store.ts` | Singleton `DashboardState` + `subscribe()` / `dispatch()` |
| `src/main.ts` | App entry: mount components, wire events, load bundled data |
| `src/style.css` | Design tokens (CSS vars), layout, component styles |
| `src/charts/heatmap.ts` | Plotly Heatmap trace for cohort overview |
| `src/charts/vafBar.ts` | Horizontal VAF bar chart per sample |
| `src/charts/coverage.ts` | Read depth bar chart per target |
| `src/components/upload.ts` | File picker + drag-drop handler |
| `src/components/table.ts` | Sortable/searchable summary table |
| `src/components/sampleCard.ts` | Per-sample clinical card with therapy badge |
| `src/components/filters.ts` | Filter panel (cohort, AR-V7, minReads) |
| `tests/parser.test.ts` | Unit tests for all parser functions |
| `tests/store.test.ts` | Unit tests for state dispatch and subscriptions |

---

## Task 1: Project Scaffolding

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `vitest.config.ts`
- Create: `index.html`

- [ ] **Step 1.1: Init package.json and install deps**

```bash
cd /home/simone/work/AR_mira_mutation
npm init -y
npm install --save-dev vite typescript vitest @vitest/ui
npm install plotly.js-dist-min
```

Expected: `node_modules/` created, `package.json` has deps.

- [ ] **Step 1.2: Write vite.config.ts**

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  resolve: {
    alias: { '@': resolve(__dirname, 'src') },
  },
});
```

- [ ] **Step 1.3: Write tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "paths": { "@/*": ["./src/*"] },
    "types": ["vitest/globals"]
  },
  "include": ["src", "tests"]
}
```

- [ ] **Step 1.4: Write vitest.config.ts**

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
  },
});
```

- [ ] **Step 1.5: Update package.json scripts**

Edit `package.json` scripts section:
```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest"
  }
}
```

- [ ] **Step 1.6: Write index.html shell**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>MIRA AR Dashboard</title>
  <link rel="stylesheet" href="/src/style.css" />
</head>
<body>
  <div id="app">
    <aside id="sidebar">
      <div id="upload-zone"></div>
      <div id="filter-panel"></div>
    </aside>
    <main id="main-content">
      <section id="heatmap-section">
        <h2 class="section-title">Cohort Overview</h2>
        <div id="heatmap-chart"></div>
      </section>
      <section id="table-section">
        <h2 class="section-title">Sample Summary</h2>
        <div id="summary-table"></div>
      </section>
    </main>
    <aside id="detail-drawer" class="drawer--closed">
      <div id="sample-card"></div>
    </aside>
  </div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

- [ ] **Step 1.7: Verify dev server starts**

```bash
npm run dev
```
Expected: Vite outputs `Local: http://localhost:5173/` with no errors.

- [ ] **Step 1.8: Commit**

```bash
git init
git add package.json vite.config.ts tsconfig.json vitest.config.ts index.html
git commit -m "feat: scaffold Vite + TypeScript + Vitest project"
```

---

## Task 2: Types & Parser (TDD)

**Files:**
- Create: `src/types.ts`
- Create: `src/parser.ts`
- Create: `tests/parser.test.ts`

- [ ] **Step 2.1: Write src/types.ts**

```typescript
// src/types.ts
export type Call = 'PRESENCE' | 'ABSENCE' | 'MUT' | 'WT';
export type TargetType = 'SPLICE_JUNCTION' | 'SNP';
export type Cohort = 'WCDT' | 'TCGA' | 'UNKNOWN';
export type TherapyRec = 'Taxane' | 'Darolutamide' | 'Continue ARPI' | 'Uncertain';

export interface MiraRecord {
  sample: string;
  target: string;
  type: TargetType;
  call: Call;
  totalReads: number;
  altReads: number | null;
  vaf: number | null;
}

export interface SampleSummary {
  sampleId: string;
  cohort: Cohort;
  records: MiraRecord[];
  therapyRecommendation: TherapyRec;
  alterationCount: number;
  hasLowReads: boolean;
}

export interface DashboardState {
  samples: SampleSummary[];
  selectedSample: string | null;
  filters: {
    cohort: Cohort | 'ALL';
    arv7: 'ALL' | 'PRESENCE' | 'ABSENCE';
    minReads: number;
  };
}
```

- [ ] **Step 2.2: Write failing tests for parser**

```typescript
// tests/parser.test.ts
import { describe, it, expect } from 'vitest';
import { parseSummaryTsv, inferCohort, inferTherapy, buildSampleSummary } from '@/parser';

const SAMPLE_TSV = `sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf
DTB-034-BL-T-RNA_R1\tAR_V7_exon3_CE3_junction\tSPLICE_JUNCTION\tPRESENCE\t1414\t.\t.
DTB-034-BL-T-RNA_R1\tAR_CE3_full\tSPLICE_JUNCTION\tPRESENCE\t1230\t.\t.
DTB-034-BL-T-RNA_R1\tAR_T878A_region\tSNP\tWT\t15054\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_L702H_region\tSNP\tWT\t3814\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_W742C_region\tSNP\tWT\t5080\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_H875Y_region\tSNP\tWT\t14720\t0\t0.0000`;

describe('parseSummaryTsv', () => {
  it('parses all 6 rows', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records).toHaveLength(6);
  });

  it('sets vaf null for splice targets', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records[0].vaf).toBeNull();
    expect(records[0].altReads).toBeNull();
  });

  it('parses SNP vaf as float', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records[2].vaf).toBe(0.0);
    expect(records[2].altReads).toBe(0);
  });

  it('parses totalReads as integer', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records[0].totalReads).toBe(1414);
  });
});

describe('inferCohort', () => {
  it('identifies WCDT by DTB- prefix', () => expect(inferCohort('DTB-034-BL')).toBe('WCDT'));
  it('identifies TCGA by TCGA- prefix', () => expect(inferCohort('TCGA-EJ-7314')).toBe('TCGA'));
  it('returns UNKNOWN for unrecognized prefixes', () => expect(inferCohort('SAMPLE-001')).toBe('UNKNOWN'));
});

describe('inferTherapy', () => {
  it('returns Taxane when AR-V7 is PRESENCE', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(inferTherapy(records)).toBe('Taxane');
  });

  it('returns Darolutamide when T878A is MUT', () => {
    const tsv = SAMPLE_TSV.replace('PRESENCE\t1414', 'ABSENCE\t1414')
      .replace('PRESENCE\t1230', 'ABSENCE\t1230')
      .replace('WT\t15054\t0\t0.0000', 'MUT\t15054\t150\t0.0100');
    expect(inferTherapy(parseSummaryTsv(tsv))).toBe('Darolutamide');
  });

  it('returns Continue ARPI when all WT/ABSENCE', () => {
    const tsv = SAMPLE_TSV.replace('PRESENCE\t1414', 'ABSENCE\t1414')
      .replace('PRESENCE\t1230', 'ABSENCE\t1230');
    expect(inferTherapy(parseSummaryTsv(tsv))).toBe('Continue ARPI');
  });
});

describe('buildSampleSummary', () => {
  it('flags hasLowReads when any target has < 10 reads', () => {
    const lowTsv = SAMPLE_TSV.replace('WT\t3814', 'WT\t5');
    const summary = buildSampleSummary(parseSummaryTsv(lowTsv));
    expect(summary.hasLowReads).toBe(true);
  });

  it('counts alterationCount as MUT + PRESENCE calls', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    const summary = buildSampleSummary(records);
    expect(summary.alterationCount).toBe(2); // 2 PRESENCE (V7 + CE3)
  });
});
```

- [ ] **Step 2.3: Run tests — verify they fail**

```bash
npm test
```
Expected: FAIL — `@/parser` module not found.

- [ ] **Step 2.4: Write src/parser.ts**

```typescript
// src/parser.ts
import type { MiraRecord, TargetType, Call, Cohort, TherapyRec, SampleSummary } from './types';

export function parseSummaryTsv(raw: string): MiraRecord[] {
  const lines = raw.trim().split('\n');
  return lines.slice(1).map(line => {
    const cols = line.split('\t');
    return {
      sample: cols[0],
      target: cols[1],
      type: cols[2] as TargetType,
      call: cols[3] as Call,
      totalReads: parseInt(cols[4]) || 0,
      altReads: cols[5] === '.' ? null : parseInt(cols[5]),
      vaf: cols[6] === '.' ? null : parseFloat(cols[6]),
    };
  });
}

export function inferCohort(sampleId: string): Cohort {
  if (sampleId.startsWith('DTB-')) return 'WCDT';
  if (sampleId.startsWith('TCGA-')) return 'TCGA';
  return 'UNKNOWN';
}

export function inferTherapy(records: MiraRecord[]): TherapyRec {
  const get = (target: string) => records.find(r => r.target.includes(target))?.call;
  if (get('V7_exon3') === 'PRESENCE') return 'Taxane';
  if (get('T878A') === 'MUT' || get('L702H') === 'MUT') return 'Darolutamide';
  if (records.every(r => r.call === 'WT' || r.call === 'ABSENCE')) return 'Continue ARPI';
  return 'Uncertain';
}

export function buildSampleSummary(records: MiraRecord[]): SampleSummary {
  const sampleId = records[0]?.sample ?? 'unknown';
  return {
    sampleId,
    cohort: inferCohort(sampleId),
    records,
    therapyRecommendation: inferTherapy(records),
    alterationCount: records.filter(r => r.call === 'MUT' || r.call === 'PRESENCE').length,
    hasLowReads: records.some(r => r.totalReads < 10),
  };
}
```

- [ ] **Step 2.5: Run tests — verify they pass**

```bash
npm test
```
Expected: All tests PASS.

- [ ] **Step 2.6: Commit**

```bash
git add src/types.ts src/parser.ts tests/parser.test.ts
git commit -m "feat: add types, parser, and unit tests"
```

---

## Task 3: State Store (TDD)

**Files:**
- Create: `src/store.ts`
- Create: `tests/store.test.ts`

- [ ] **Step 3.1: Write failing store tests**

```typescript
// tests/store.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { store, dispatch, subscribe, getFiltered } from '@/store';

beforeEach(() => {
  dispatch({ type: 'RESET' });
});

describe('store dispatch', () => {
  it('starts with empty samples', () => {
    expect(store.samples).toHaveLength(0);
  });

  it('ADD_SAMPLES replaces sample list', () => {
    dispatch({ type: 'ADD_SAMPLES', samples: [{ sampleId: 'DTB-001', cohort: 'WCDT', records: [], therapyRecommendation: 'Uncertain', alterationCount: 0, hasLowReads: false }] });
    expect(store.samples).toHaveLength(1);
  });

  it('SELECT_SAMPLE sets selectedSample', () => {
    dispatch({ type: 'SELECT_SAMPLE', sampleId: 'DTB-001' });
    expect(store.selectedSample).toBe('DTB-001');
  });

  it('SET_FILTER updates cohort filter', () => {
    dispatch({ type: 'SET_FILTER', filter: { cohort: 'WCDT' } });
    expect(store.filters.cohort).toBe('WCDT');
  });
});

describe('subscribe', () => {
  it('calls subscriber on every dispatch', () => {
    let callCount = 0;
    const unsub = subscribe(() => callCount++);
    dispatch({ type: 'SELECT_SAMPLE', sampleId: null });
    expect(callCount).toBe(1);
    unsub();
    dispatch({ type: 'SELECT_SAMPLE', sampleId: null });
    expect(callCount).toBe(1); // unsubscribed — no more calls
  });
});

describe('getFiltered', () => {
  it('filters by cohort', () => {
    dispatch({
      type: 'ADD_SAMPLES', samples: [
        { sampleId: 'DTB-001', cohort: 'WCDT', records: [], therapyRecommendation: 'Uncertain', alterationCount: 0, hasLowReads: false },
        { sampleId: 'TCGA-001', cohort: 'TCGA', records: [], therapyRecommendation: 'Uncertain', alterationCount: 0, hasLowReads: false },
      ]
    });
    dispatch({ type: 'SET_FILTER', filter: { cohort: 'WCDT' } });
    expect(getFiltered()).toHaveLength(1);
    expect(getFiltered()[0].sampleId).toBe('DTB-001');
  });
});
```

- [ ] **Step 3.2: Run tests — verify they fail**

```bash
npm test
```
Expected: FAIL — `@/store` not found.

- [ ] **Step 3.3: Write src/store.ts**

```typescript
// src/store.ts
import type { DashboardState, SampleSummary, Cohort } from './types';

type Action =
  | { type: 'ADD_SAMPLES'; samples: SampleSummary[] }
  | { type: 'SELECT_SAMPLE'; sampleId: string | null }
  | { type: 'SET_FILTER'; filter: Partial<DashboardState['filters']> }
  | { type: 'RESET' };

export const store: DashboardState = {
  samples: [],
  selectedSample: null,
  filters: { cohort: 'ALL', arv7: 'ALL', minReads: 0 },
};

const subscribers = new Set<() => void>();

export function subscribe(fn: () => void): () => void {
  subscribers.add(fn);
  return () => subscribers.delete(fn);
}

export function dispatch(action: Action): void {
  switch (action.type) {
    case 'ADD_SAMPLES':
      store.samples = action.samples;
      break;
    case 'SELECT_SAMPLE':
      store.selectedSample = action.sampleId;
      break;
    case 'SET_FILTER':
      Object.assign(store.filters, action.filter);
      break;
    case 'RESET':
      store.samples = [];
      store.selectedSample = null;
      store.filters = { cohort: 'ALL', arv7: 'ALL', minReads: 0 };
      break;
  }
  subscribers.forEach(fn => fn());
}

export function getFiltered(): SampleSummary[] {
  const { cohort, arv7, minReads } = store.filters;
  return store.samples.filter(s => {
    if (cohort !== 'ALL' && s.cohort !== cohort) return false;
    if (arv7 !== 'ALL') {
      const v7 = s.records.find(r => r.target.includes('V7_exon3'));
      if (v7?.call !== arv7) return false;
    }
    if (minReads > 0 && s.records.some(r => r.totalReads < minReads)) return false;
    return true;
  });
}
```

- [ ] **Step 3.4: Run tests — verify all pass**

```bash
npm test
```
Expected: All PASS.

- [ ] **Step 3.5: Commit**

```bash
git add src/store.ts tests/store.test.ts
git commit -m "feat: add reactive store with dispatch/subscribe"
```

---

## Task 4: CSS Design System

**Files:**
- Create: `src/style.css`

- [ ] **Step 4.1: Write complete style.css**

```css
/* src/style.css */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap');

/* ── Design Tokens ─────────────────────────────────────────── */
:root {
  --bg:          #0d1117;
  --surface:     #161b22;
  --surface-2:   #1c2128;
  --border:      #30363d;
  --text:        #e6edf3;
  --text-muted:  #8b949e;
  --red:         #e63946;
  --blue:        #457b9d;
  --amber:       #f4a261;
  --green:       #3fb950;
  --radius:      6px;
  --transition:  150ms ease;
  --font-ui:     'Inter', system-ui, sans-serif;
  --font-mono:   'JetBrains Mono', 'Fira Code', monospace;
  --sidebar-w:   260px;
  --drawer-w:    360px;
}

/* ── Reset ─────────────────────────────────────────────────── */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: var(--font-ui);
  font-size: 14px;
  background: var(--bg);
  color: var(--text);
  line-height: 1.5;
  min-height: 100vh;
}

/* ── Layout ─────────────────────────────────────────────────── */
#app {
  display: grid;
  grid-template-columns: var(--sidebar-w) 1fr;
  grid-template-rows: 100vh;
  grid-template-areas: "sidebar main";
  overflow: hidden;
}

#app.drawer-open {
  grid-template-columns: var(--sidebar-w) 1fr var(--drawer-w);
  grid-template-areas: "sidebar main drawer";
}

#sidebar {
  grid-area: sidebar;
  background: var(--surface);
  border-right: 1px solid var(--border);
  padding: 16px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

#main-content {
  grid-area: main;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 32px;
}

#detail-drawer {
  grid-area: drawer;
  background: var(--surface);
  border-left: 1px solid var(--border);
  overflow-y: auto;
  padding: 20px;
  transform: translateX(0);
  transition: transform var(--transition);
}

#detail-drawer.drawer--closed { display: none; }

/* ── Section Titles ─────────────────────────────────────────── */
.section-title {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-bottom: 12px;
}

/* ── Upload Zone ─────────────────────────────────────────────── */
.upload-zone {
  border: 1px dashed var(--border);
  border-radius: var(--radius);
  padding: 20px 12px;
  text-align: center;
  cursor: pointer;
  transition: border-color var(--transition), background var(--transition);
}

.upload-zone:hover,
.upload-zone.drag-over {
  border-color: var(--blue);
  background: color-mix(in srgb, var(--blue) 8%, transparent);
}

.upload-zone input[type="file"] { display: none; }

.upload-zone__icon {
  width: 32px;
  height: 32px;
  margin: 0 auto 8px;
  color: var(--text-muted);
}

.upload-zone__label {
  display: block;
  font-size: 13px;
  color: var(--text-muted);
}

.upload-zone__label strong { color: var(--blue); }

.upload-count {
  margin-top: 8px;
  font-size: 12px;
  font-family: var(--font-mono);
  color: var(--green);
}

/* ── Filters ─────────────────────────────────────────────────── */
.filter-group { display: flex; flex-direction: column; gap: 8px; }

.filter-label {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.filter-select,
.filter-input {
  width: 100%;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 6px 10px;
  outline: none;
  transition: border-color var(--transition);
  cursor: pointer;
}

.filter-select:focus,
.filter-input:focus {
  border-color: var(--blue);
}

/* ── Call Badges ─────────────────────────────────────────────── */
.badge {
  display: inline-block;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  padding: 2px 7px;
  border-radius: 4px;
  letter-spacing: 0.04em;
}

.badge--mut, .badge--presence {
  background: color-mix(in srgb, var(--red) 20%, transparent);
  color: var(--red);
}

.badge--wt, .badge--absence {
  background: color-mix(in srgb, var(--blue) 20%, transparent);
  color: var(--blue);
}

.badge--warning {
  background: color-mix(in srgb, var(--amber) 20%, transparent);
  color: var(--amber);
}

/* ── Therapy Badge ───────────────────────────────────────────── */
.therapy-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  padding: 6px 12px;
  border-radius: var(--radius);
}

.therapy-badge--taxane   { background: color-mix(in srgb, var(--red) 15%, transparent); color: var(--red); border: 1px solid color-mix(in srgb, var(--red) 30%, transparent); }
.therapy-badge--darolu   { background: color-mix(in srgb, var(--amber) 15%, transparent); color: var(--amber); border: 1px solid color-mix(in srgb, var(--amber) 30%, transparent); }
.therapy-badge--arpi     { background: color-mix(in srgb, var(--green) 15%, transparent); color: var(--green); border: 1px solid color-mix(in srgb, var(--green) 30%, transparent); }
.therapy-badge--uncertain { background: var(--surface-2); color: var(--text-muted); border: 1px solid var(--border); }

/* ── Summary Table ───────────────────────────────────────────── */
.summary-table-wrap {
  overflow-x: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius);
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

thead tr {
  background: var(--surface-2);
  border-bottom: 1px solid var(--border);
}

th {
  padding: 10px 12px;
  text-align: left;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-muted);
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
}

th:hover { color: var(--text); }
th .sort-icon { margin-left: 4px; opacity: 0.4; }
th.sorted .sort-icon { opacity: 1; color: var(--blue); }

tbody tr {
  border-bottom: 1px solid var(--border);
  transition: background var(--transition);
  cursor: pointer;
}

tbody tr:last-child { border-bottom: none; }
tbody tr:hover { background: var(--surface-2); }
tbody tr.selected { background: color-mix(in srgb, var(--blue) 10%, transparent); }

td {
  padding: 9px 12px;
  font-family: var(--font-mono);
  font-size: 12px;
}

td.sample-id { color: var(--text); font-size: 11px; }
td.cohort { color: var(--text-muted); font-size: 11px; }

/* ── Table Search + Export Bar ───────────────────────────────── */
.table-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.search-input {
  flex: 1;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 13px;
  padding: 6px 10px;
  outline: none;
  transition: border-color var(--transition);
}

.search-input:focus { border-color: var(--blue); }
.search-input::placeholder { color: var(--text-muted); }

.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text);
  font-size: 12px;
  font-weight: 500;
  padding: 6px 12px;
  cursor: pointer;
  transition: background var(--transition), border-color var(--transition);
}

.btn:hover { background: var(--surface); border-color: var(--text-muted); }
.btn:focus-visible { outline: 2px solid var(--blue); outline-offset: 2px; }

/* ── Sample Card ─────────────────────────────────────────────── */
.sample-card__header {
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border);
}

.sample-card__id {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  margin-bottom: 8px;
}

.sample-card__section {
  margin-top: 16px;
}

.sample-card__section-title {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.records-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.record-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 8px;
  background: var(--surface-2);
  border-radius: var(--radius);
  font-size: 12px;
}

.record-row__target {
  font-family: var(--font-mono);
  color: var(--text-muted);
  font-size: 11px;
}

.record-row__meta {
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.low-reads-warning {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  background: color-mix(in srgb, var(--amber) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--amber) 30%, transparent);
  border-radius: var(--radius);
  font-size: 12px;
  color: var(--amber);
  margin-top: 12px;
}

.close-btn {
  position: absolute;
  top: 16px;
  right: 16px;
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius);
  transition: color var(--transition);
  line-height: 1;
}

.close-btn:hover { color: var(--text); }
.close-btn:focus-visible { outline: 2px solid var(--blue); outline-offset: 2px; }

#detail-drawer { position: relative; }

/* ── Empty State ─────────────────────────────────────────────── */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;
  color: var(--text-muted);
  gap: 12px;
}

.empty-state__icon { width: 40px; height: 40px; opacity: 0.4; }
.empty-state__text { font-size: 13px; }

/* ── Motion ──────────────────────────────────────────────────── */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { transition: none !important; animation: none !important; }
}

/* ── Focus ───────────────────────────────────────────────────── */
:focus-visible { outline: 2px solid var(--blue); outline-offset: 2px; }
```

- [ ] **Step 4.2: Verify dev server shows styled empty shell**

```bash
npm run dev
```
Open browser at `http://localhost:5173` — should see dark sidebar + main area with no errors.

- [ ] **Step 4.3: Commit**

```bash
git add src/style.css
git commit -m "feat: add design system CSS with tokens and all component styles"
```

---

## Task 5: Upload Component

**Files:**
- Create: `src/components/upload.ts`

- [ ] **Step 5.1: Write src/components/upload.ts**

```typescript
// src/components/upload.ts
import { parseSummaryTsv, buildSampleSummary } from '@/parser';
import { dispatch } from '@/store';

const UPLOAD_ICON = `<svg class="upload-zone__icon" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
  <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"/>
</svg>`;

export function mountUpload(container: HTMLElement): void {
  container.innerHTML = `
    <div class="upload-zone" id="drop-zone" role="button" tabindex="0" aria-label="Upload summary TSV files">
      ${UPLOAD_ICON}
      <label class="upload-zone__label">
        <strong>Choose files</strong> or drag & drop<br>
        <span style="font-size:11px">*_AR.summary.tsv</span>
      </label>
      <input type="file" id="file-input" accept=".tsv" multiple />
    </div>
    <div class="upload-count" id="upload-count" aria-live="polite"></div>
  `;

  const zone = container.querySelector<HTMLDivElement>('#drop-zone')!;
  const input = container.querySelector<HTMLInputElement>('#file-input')!;
  const count = container.querySelector<HTMLDivElement>('#upload-count')!;

  zone.addEventListener('click', () => input.click());
  zone.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') input.click(); });
  zone.addEventListener('dragover', e => { e.preventDefault(); zone.classList.add('drag-over'); });
  zone.addEventListener('dragleave', () => zone.classList.remove('drag-over'));
  zone.addEventListener('drop', e => {
    e.preventDefault();
    zone.classList.remove('drag-over');
    if (e.dataTransfer?.files) processFiles(Array.from(e.dataTransfer.files), count);
  });
  input.addEventListener('change', () => {
    if (input.files) processFiles(Array.from(input.files), count);
  });
}

async function processFiles(files: File[], countEl: HTMLElement): Promise<void> {
  const summaryFiles = files.filter(f => f.name.endsWith('.summary.tsv'));
  if (!summaryFiles.length) return;

  const summaries = await Promise.all(
    summaryFiles.map(async f => {
      const raw = await f.text();
      return buildSampleSummary(parseSummaryTsv(raw));
    })
  );

  dispatch({ type: 'ADD_SAMPLES', samples: summaries });
  countEl.textContent = `${summaries.length} sample${summaries.length !== 1 ? 's' : ''} loaded`;
}
```

- [ ] **Step 5.2: Mount upload in main.ts (minimal wiring)**

```typescript
// src/main.ts
import '@/style.css';
import { mountUpload } from '@/components/upload';

mountUpload(document.querySelector('#upload-zone')!);
```

- [ ] **Step 5.3: Verify in browser**

```bash
npm run dev
```
Open `http://localhost:5173` — drag a `*_AR.summary.tsv` file onto the upload zone. Confirm "1 sample loaded" appears.

- [ ] **Step 5.4: Commit**

```bash
git add src/components/upload.ts src/main.ts
git commit -m "feat: file upload and drag-drop with TSV parsing"
```

---

## Task 6: Cohort Heatmap

**Files:**
- Create: `src/charts/heatmap.ts`

- [ ] **Step 6.1: Install Plotly types**

```bash
npm install --save-dev @types/plotly.js
```

- [ ] **Step 6.2: Write src/charts/heatmap.ts**

```typescript
// src/charts/heatmap.ts
import Plotly from 'plotly.js-dist-min';
import type { SampleSummary } from '@/types';
import { dispatch } from '@/store';

const TARGETS = [
  { key: 'V7_exon3', label: 'AR-V7' },
  { key: 'CE3_full', label: 'CE3' },
  { key: 'T878A',    label: 'T878A' },
  { key: 'L702H',    label: 'L702H' },
  { key: 'W742C',    label: 'W742C' },
  { key: 'H875Y',    label: 'H875Y' },
];

function callToZ(call: string | undefined): number {
  if (call === 'MUT' || call === 'PRESENCE') return 1;
  if (call === 'WT'  || call === 'ABSENCE')  return 0;
  return -1; // missing
}

export function renderHeatmap(container: HTMLElement, samples: SampleSummary[]): void {
  if (!samples.length) {
    container.innerHTML = `<div class="empty-state">
      <svg class="empty-state__icon" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 3h18v18H3z"/></svg>
      <p class="empty-state__text">Load summary TSV files to see the heatmap</p>
    </div>`;
    return;
  }

  const z = samples.map(s =>
    TARGETS.map(t => {
      const rec = s.records.find(r => r.target.includes(t.key));
      return callToZ(rec?.call);
    })
  );

  const text = samples.map(s =>
    TARGETS.map(t => {
      const rec = s.records.find(r => r.target.includes(t.key));
      if (!rec) return 'No data';
      const vafStr = rec.vaf !== null ? ` | VAF: ${(rec.vaf * 100).toFixed(1)}%` : '';
      const readsStr = ` | Reads: ${rec.totalReads}`;
      const warn = rec.totalReads < 10 ? ' ⚠ LOW READS' : '';
      return `${s.sampleId}<br>${rec.target}<br><b>${rec.call}</b>${vafStr}${readsStr}${warn}`;
    })
  );

  const trace: Partial<Plotly.PlotData> = {
    type: 'heatmap',
    z,
    x: TARGETS.map(t => t.label),
    y: samples.map(s => s.sampleId),
    text: text as unknown as string[],
    hovertemplate: '%{text}<extra></extra>',
    colorscale: [
      [-1, '#30363d'],
      [0,  '#457b9d'],
      [1,  '#e63946'],
    ] as unknown as Plotly.ColorScale,
    zmin: -1, zmax: 1,
    showscale: false,
    xgap: 2,
    ygap: 2,
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor:  'transparent',
    font: { family: 'JetBrains Mono, monospace', color: '#e6edf3', size: 11 },
    margin: { l: 180, r: 20, t: 20, b: 60 },
    xaxis: { fixedrange: true, tickfont: { size: 11 } },
    yaxis: { fixedrange: true, tickfont: { size: 10, family: 'JetBrains Mono, monospace' } },
    height: Math.max(300, samples.length * 32 + 80),
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });

  container.on('plotly_click', (data: Plotly.PlotMouseEvent) => {
    const pt = data.points[0];
    const sampleId = pt.y as string;
    dispatch({ type: 'SELECT_SAMPLE', sampleId });
  });
}
```

- [ ] **Step 6.3: Wire heatmap into main.ts**

```typescript
// src/main.ts
import '@/style.css';
import { mountUpload } from '@/components/upload';
import { renderHeatmap } from '@/charts/heatmap';
import { subscribe, getFiltered } from '@/store';

mountUpload(document.querySelector('#upload-zone')!);

const heatmapEl = document.querySelector<HTMLElement>('#heatmap-chart')!;
renderHeatmap(heatmapEl, []);

subscribe(() => {
  renderHeatmap(heatmapEl, getFiltered());
});
```

- [ ] **Step 6.4: Verify heatmap renders after loading files**

Load all 20 `*_AR.summary.tsv` files from `tables/` via the upload zone. Heatmap should show 20 rows × 6 columns.

- [ ] **Step 6.5: Commit**

```bash
git add src/charts/heatmap.ts src/main.ts
git commit -m "feat: cohort heatmap with Plotly, colored by call, click-to-select"
```

---

## Task 7: Summary Table

**Files:**
- Create: `src/components/table.ts`

- [ ] **Step 7.1: Write src/components/table.ts**

```typescript
// src/components/table.ts
import type { SampleSummary } from '@/types';
import { dispatch } from '@/store';

const TARGETS = ['V7_exon3','CE3_full','T878A','L702H','W742C','H875Y'];
const LABELS  = ['AR-V7','CE3','T878A','L702H','W742C','H875Y'];

type SortKey = 'sampleId' | 'cohort' | 'alterationCount' | 'therapyRecommendation';

let sortKey: SortKey = 'alterationCount';
let sortAsc = false;
let searchQuery = '';

function callBadge(call: string | undefined): string {
  if (!call) return '<span class="badge badge--warning">?</span>';
  const cls = call === 'MUT' || call === 'PRESENCE' ? 'mut' : 'wt';
  return `<span class="badge badge--${cls}">${call}</span>`;
}

function therapyClass(rec: SampleSummary['therapyRecommendation']): string {
  return { Taxane: 'taxane', Darolutamide: 'darolu', 'Continue ARPI': 'arpi', Uncertain: 'uncertain' }[rec];
}

export function renderTable(container: HTMLElement, samples: SampleSummary[], selectedId: string | null): void {
  const filtered = samples.filter(s =>
    s.sampleId.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const sorted = [...filtered].sort((a, b) => {
    const av = a[sortKey]; const bv = b[sortKey];
    const cmp = typeof av === 'number' ? (av - (bv as number)) : String(av).localeCompare(String(bv));
    return sortAsc ? cmp : -cmp;
  });

  container.innerHTML = `
    <div class="table-toolbar">
      <input class="search-input" type="search" placeholder="Search sample ID…" value="${searchQuery}" aria-label="Search samples">
      <button class="btn" id="export-csv" aria-label="Export CSV">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>
        Export CSV
      </button>
    </div>
    <div class="summary-table-wrap">
      <table role="grid">
        <thead>
          <tr>
            <th data-key="sampleId" scope="col">Sample ${sortKey==='sampleId'?'<span class="sort-icon">'+(sortAsc?'↑':'↓')+'</span>':''}</th>
            <th data-key="cohort" scope="col">Cohort ${sortKey==='cohort'?'<span class="sort-icon">'+(sortAsc?'↑':'↓')+'</span>':''}</th>
            ${LABELS.map(l => `<th scope="col">${l}</th>`).join('')}
            <th data-key="alterationCount" scope="col"># Alt ${sortKey==='alterationCount'?'<span class="sort-icon">'+(sortAsc?'↑':'↓')+'</span>':''}</th>
            <th data-key="therapyRecommendation" scope="col">Therapy ${sortKey==='therapyRecommendation'?'<span class="sort-icon">'+(sortAsc?'↑':'↓')+'</span>':''}</th>
          </tr>
        </thead>
        <tbody>
          ${sorted.map(s => {
            const calls = TARGETS.map(t => {
              const rec = s.records.find(r => r.target.includes(t));
              return callBadge(rec?.call);
            });
            const warnIcon = s.hasLowReads ? '<span title="Low coverage warning" style="color:var(--amber)"> ⚠</span>' : '';
            return `<tr data-id="${s.sampleId}" class="${s.sampleId === selectedId ? 'selected' : ''}" tabindex="0">
              <td class="sample-id">${s.sampleId}${warnIcon}</td>
              <td class="cohort">${s.cohort}</td>
              ${calls.map(c => `<td>${c}</td>`).join('')}
              <td style="text-align:center;font-weight:600">${s.alterationCount}</td>
              <td><span class="therapy-badge therapy-badge--${therapyClass(s.therapyRecommendation)}">${s.therapyRecommendation}</span></td>
            </tr>`;
          }).join('')}
        </tbody>
      </table>
    </div>
  `;

  container.querySelector('.search-input')!.addEventListener('input', e => {
    searchQuery = (e.target as HTMLInputElement).value;
    renderTable(container, samples, selectedId);
  });

  container.querySelectorAll('th[data-key]').forEach(th => {
    th.addEventListener('click', () => {
      const key = (th as HTMLElement).dataset.key as SortKey;
      if (sortKey === key) sortAsc = !sortAsc;
      else { sortKey = key; sortAsc = false; }
      renderTable(container, samples, selectedId);
    });
  });

  container.querySelectorAll('tbody tr').forEach(tr => {
    const id = (tr as HTMLElement).dataset.id!;
    tr.addEventListener('click', () => dispatch({ type: 'SELECT_SAMPLE', sampleId: id }));
    tr.addEventListener('keydown', e => { if ((e as KeyboardEvent).key === 'Enter') dispatch({ type: 'SELECT_SAMPLE', sampleId: id }); });
  });

  container.querySelector('#export-csv')?.addEventListener('click', () => exportCsv(sorted));
}

function exportCsv(samples: SampleSummary[]): void {
  const header = ['Sample','Cohort',...LABELS,'Alterations','Therapy'];
  const rows = samples.map(s => [
    s.sampleId, s.cohort,
    ...TARGETS.map(t => s.records.find(r => r.target.includes(t))?.call ?? ''),
    String(s.alterationCount), s.therapyRecommendation,
  ]);
  const csv = [header, ...rows].map(r => r.map(v => `"${v}"`).join(',')).join('\n');
  const blob = new Blob([csv], { type: 'text/csv' });
  const a = Object.assign(document.createElement('a'), { href: URL.createObjectURL(blob), download: 'mira-summary.csv' });
  a.click(); URL.revokeObjectURL(a.href);
}
```

- [ ] **Step 7.2: Wire table into main.ts**

```typescript
// src/main.ts
import '@/style.css';
import { mountUpload } from '@/components/upload';
import { renderHeatmap } from '@/charts/heatmap';
import { renderTable } from '@/components/table';
import { subscribe, getFiltered, store } from '@/store';

mountUpload(document.querySelector('#upload-zone')!);

const heatmapEl  = document.querySelector<HTMLElement>('#heatmap-chart')!;
const tableEl    = document.querySelector<HTMLElement>('#summary-table')!;

renderHeatmap(heatmapEl, []);
renderTable(tableEl, [], null);

subscribe(() => {
  const samples = getFiltered();
  renderHeatmap(heatmapEl, samples);
  renderTable(tableEl, samples, store.selectedSample);
});
```

- [ ] **Step 7.3: Verify table, sorting, search, CSV export**

Load all 20 files. Table should appear, columns sortable, search filtering sample IDs, CSV downloading on button click.

- [ ] **Step 7.4: Commit**

```bash
git add src/components/table.ts src/main.ts
git commit -m "feat: sortable/searchable summary table with CSV export"
```

---

## Task 8: Sample Detail Drawer

**Files:**
- Create: `src/components/sampleCard.ts`

- [ ] **Step 8.1: Write src/components/sampleCard.ts**

```typescript
// src/components/sampleCard.ts
import type { SampleSummary } from '@/types';
import { dispatch } from '@/store';

const CLOSE_ICON = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12"/></svg>`;
const WARN_ICON  = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 9v4M12 17h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/></svg>`;

function therapyClass(rec: SampleSummary['therapyRecommendation']): string {
  return { Taxane: 'taxane', Darolutamide: 'darolu', 'Continue ARPI': 'arpi', Uncertain: 'uncertain' }[rec];
}

export function renderSampleCard(drawer: HTMLElement, sample: SampleSummary | null): void {
  const app = document.querySelector<HTMLElement>('#app')!;

  if (!sample) {
    drawer.classList.add('drawer--closed');
    app.classList.remove('drawer-open');
    return;
  }

  drawer.classList.remove('drawer--closed');
  app.classList.add('drawer-open');

  drawer.innerHTML = `
    <button class="close-btn" aria-label="Close detail panel">${CLOSE_ICON}</button>
    <div class="sample-card">
      <div class="sample-card__header">
        <div class="sample-card__id">${sample.sampleId}</div>
        <span class="badge badge--${sample.cohort === 'WCDT' ? 'mut' : 'wt'}" style="font-size:10px">${sample.cohort}</span>
      </div>

      <div class="sample-card__section">
        <div class="sample-card__section-title">Therapy Recommendation</div>
        <span class="therapy-badge therapy-badge--${therapyClass(sample.therapyRecommendation)}">
          ${sample.therapyRecommendation}
        </span>
      </div>

      <div class="sample-card__section">
        <div class="sample-card__section-title">AR Calls</div>
        <div class="records-list">
          ${sample.records.map(r => {
            const callCls = (r.call === 'MUT' || r.call === 'PRESENCE') ? 'mut' : 'wt';
            const vafStr = r.vaf !== null ? `VAF ${(r.vaf * 100).toFixed(1)}%` : '';
            const readsStr = `${r.totalReads} reads`;
            const warn = r.totalReads < 10 ? `<span style="color:var(--amber)" title="Low coverage">${WARN_ICON}</span>` : '';
            return `<div class="record-row">
              <span class="record-row__target">${r.target.replace('AR_','').replace('_region','').replace('_junction','')}</span>
              <span class="badge badge--${callCls}">${r.call}</span>
              <span class="record-row__meta">${[vafStr, readsStr].filter(Boolean).join(' · ')}${warn}</span>
            </div>`;
          }).join('')}
        </div>
      </div>

      ${sample.hasLowReads ? `<div class="low-reads-warning">${WARN_ICON} One or more targets have &lt;10 reads — interpret calls with caution.</div>` : ''}
    </div>
  `;

  drawer.querySelector('.close-btn')!.addEventListener('click', () => {
    dispatch({ type: 'SELECT_SAMPLE', sampleId: null });
  });
}
```

- [ ] **Step 8.2: Wire drawer into main.ts**

```typescript
// src/main.ts
import '@/style.css';
import { mountUpload } from '@/components/upload';
import { renderHeatmap } from '@/charts/heatmap';
import { renderTable } from '@/components/table';
import { renderSampleCard } from '@/components/sampleCard';
import { subscribe, getFiltered, store } from '@/store';

mountUpload(document.querySelector('#upload-zone')!);

const heatmapEl = document.querySelector<HTMLElement>('#heatmap-chart')!;
const tableEl   = document.querySelector<HTMLElement>('#summary-table')!;
const drawerEl  = document.querySelector<HTMLElement>('#detail-drawer')!;

renderHeatmap(heatmapEl, []);
renderTable(tableEl, [], null);
renderSampleCard(drawerEl, null);

subscribe(() => {
  const samples = getFiltered();
  const selected = store.selectedSample ? store.samples.find(s => s.sampleId === store.selectedSample) ?? null : null;
  renderHeatmap(heatmapEl, samples);
  renderTable(tableEl, samples, store.selectedSample);
  renderSampleCard(drawerEl, selected);
});
```

- [ ] **Step 8.3: Verify drawer opens on heatmap click and table row click, closes on X**

Click a cell → drawer slides open with clinical card. Click X → drawer closes.

- [ ] **Step 8.4: Commit**

```bash
git add src/components/sampleCard.ts src/main.ts
git commit -m "feat: sample detail drawer with therapy recommendation and call list"
```

---

## Task 9: Filter Panel

**Files:**
- Create: `src/components/filters.ts`

- [ ] **Step 9.1: Write src/components/filters.ts**

```typescript
// src/components/filters.ts
import { dispatch, store } from '@/store';

export function mountFilters(container: HTMLElement): void {
  container.innerHTML = `
    <div style="margin-bottom:4px;font-size:11px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;color:var(--text-muted)">Filters</div>

    <div class="filter-group">
      <label class="filter-label" for="filter-cohort">Cohort</label>
      <select class="filter-select" id="filter-cohort" aria-label="Filter by cohort">
        <option value="ALL">All cohorts</option>
        <option value="WCDT">WCDT (mCRPC)</option>
        <option value="TCGA">TCGA-PRAD</option>
      </select>
    </div>

    <div class="filter-group">
      <label class="filter-label" for="filter-arv7">AR-V7 Status</label>
      <select class="filter-select" id="filter-arv7" aria-label="Filter by AR-V7 status">
        <option value="ALL">All</option>
        <option value="PRESENCE">AR-V7+</option>
        <option value="ABSENCE">AR-V7−</option>
      </select>
    </div>

    <div class="filter-group">
      <label class="filter-label" for="filter-reads">Min Reads</label>
      <input class="filter-input" type="number" id="filter-reads" min="0" value="0" aria-label="Minimum reads threshold">
    </div>
  `;

  container.querySelector<HTMLSelectElement>('#filter-cohort')!.addEventListener('change', e => {
    dispatch({ type: 'SET_FILTER', filter: { cohort: (e.target as HTMLSelectElement).value as 'ALL' | 'WCDT' | 'TCGA' } });
  });

  container.querySelector<HTMLSelectElement>('#filter-arv7')!.addEventListener('change', e => {
    dispatch({ type: 'SET_FILTER', filter: { arv7: (e.target as HTMLSelectElement).value as 'ALL' | 'PRESENCE' | 'ABSENCE' } });
  });

  container.querySelector<HTMLInputElement>('#filter-reads')!.addEventListener('input', e => {
    dispatch({ type: 'SET_FILTER', filter: { minReads: parseInt((e.target as HTMLInputElement).value) || 0 } });
  });
}
```

- [ ] **Step 9.2: Wire filters into main.ts**

Add `import { mountFilters } from '@/components/filters';` and `mountFilters(document.querySelector('#filter-panel')!);` to `src/main.ts` at the top of the mount section.

- [ ] **Step 9.3: Verify filters update heatmap and table reactively**

Select "WCDT" cohort — table and heatmap show only 10 WCDT samples.

- [ ] **Step 9.4: Commit**

```bash
git add src/components/filters.ts src/main.ts
git commit -m "feat: filter panel for cohort, AR-V7 status, and read depth"
```

---

## Task 10: VAF Bar Chart + Read Depth Chart (in Sample Card)

**Files:**
- Create: `src/charts/vafBar.ts`
- Create: `src/charts/coverage.ts`
- Modify: `src/components/sampleCard.ts`

- [ ] **Step 10.1: Write src/charts/vafBar.ts**

```typescript
// src/charts/vafBar.ts
import Plotly from 'plotly.js-dist-min';
import type { MiraRecord } from '@/types';

export function renderVafBar(container: HTMLElement, records: MiraRecord[]): void {
  const snpRecords = records.filter(r => r.type === 'SNP');
  if (!snpRecords.length) { container.innerHTML = '<p style="color:var(--text-muted);font-size:12px">No SNP targets</p>'; return; }

  const colors = snpRecords.map(r => (r.vaf ?? 0) >= 0.3 ? '#e63946' : '#457b9d');

  const trace: Partial<Plotly.PlotData> = {
    type: 'bar',
    orientation: 'h',
    x: snpRecords.map(r => r.vaf ?? 0),
    y: snpRecords.map(r => r.target.replace('AR_','').replace('_region','')),
    marker: { color: colors },
    hovertemplate: '%{y}: %{x:.1%}<extra></extra>',
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor: 'transparent',
    font: { color: '#e6edf3', size: 10, family: 'JetBrains Mono, monospace' },
    margin: { l: 90, r: 20, t: 10, b: 30 },
    height: 140,
    xaxis: { range: [0, 1], tickformat: '.0%', gridcolor: '#30363d', fixedrange: true },
    yaxis: { fixedrange: true },
    shapes: [{ type: 'line', x0: 0.3, x1: 0.3, y0: -0.5, y1: snpRecords.length - 0.5, line: { color: '#f4a261', dash: 'dot', width: 1 } }],
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });
}
```

- [ ] **Step 10.2: Write src/charts/coverage.ts**

```typescript
// src/charts/coverage.ts
import Plotly from 'plotly.js-dist-min';
import type { MiraRecord } from '@/types';

export function renderCoverage(container: HTMLElement, records: MiraRecord[]): void {
  const colors = records.map(r => r.totalReads < 10 ? '#f4a261' : '#457b9d');

  const trace: Partial<Plotly.PlotData> = {
    type: 'bar',
    orientation: 'h',
    x: records.map(r => r.totalReads),
    y: records.map(r => r.target.replace('AR_','').replace('_region','').replace('_junction','')),
    marker: { color: colors },
    hovertemplate: '%{y}: %{x} reads<extra></extra>',
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor: 'transparent',
    font: { color: '#e6edf3', size: 10, family: 'JetBrains Mono, monospace' },
    margin: { l: 90, r: 20, t: 10, b: 30 },
    height: 160,
    xaxis: { gridcolor: '#30363d', fixedrange: true },
    yaxis: { fixedrange: true },
    shapes: [{ type: 'line', x0: 10, x1: 10, y0: -0.5, y1: records.length - 0.5, line: { color: '#f4a261', dash: 'dot', width: 1 } }],
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });
}
```

- [ ] **Step 10.3: Add chart containers to sampleCard.ts**

In the `drawer.innerHTML` template in `renderSampleCard`, after the records-list section add:

```html
<div class="sample-card__section">
  <div class="sample-card__section-title">VAF (SNP targets)</div>
  <div id="vaf-chart"></div>
</div>
<div class="sample-card__section">
  <div class="sample-card__section-title">Read Depth</div>
  <div id="coverage-chart"></div>
</div>
```

Then after `innerHTML` assignment, add:
```typescript
import { renderVafBar } from '@/charts/vafBar';
import { renderCoverage } from '@/charts/coverage';

// after drawer.innerHTML = ...
renderVafBar(drawer.querySelector<HTMLElement>('#vaf-chart')!, sample.records);
renderCoverage(drawer.querySelector<HTMLElement>('#coverage-chart')!, sample.records);
```

- [ ] **Step 10.4: Verify charts appear in detail drawer**

Click any sample — VAF bars and read depth bars appear in the drawer.

- [ ] **Step 10.5: Commit**

```bash
git add src/charts/vafBar.ts src/charts/coverage.ts src/components/sampleCard.ts
git commit -m "feat: VAF bar chart and read depth chart in sample detail drawer"
```

---

## Task 11: Load Bundled Data + Final Integration

**Files:**
- Create: `src/data-loader.ts`
- Modify: `src/main.ts`

- [ ] **Step 11.1: Write src/data-loader.ts to auto-load tables/**

```typescript
// src/data-loader.ts
import { parseSummaryTsv, buildSampleSummary } from '@/parser';
import type { SampleSummary } from '@/types';

// Vite glob import: all summary TSVs in tables/
const modules = import.meta.glob('/tables/*_AR.summary.tsv', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

export async function loadBundledData(): Promise<SampleSummary[]> {
  return Object.values(modules).map(raw => buildSampleSummary(parseSummaryTsv(raw)));
}
```

- [ ] **Step 11.2: Wire auto-load into main.ts**

```typescript
// src/main.ts (final version)
import '@/style.css';
import { mountUpload } from '@/components/upload';
import { mountFilters } from '@/components/filters';
import { renderHeatmap } from '@/charts/heatmap';
import { renderTable } from '@/components/table';
import { renderSampleCard } from '@/components/sampleCard';
import { subscribe, getFiltered, store, dispatch } from '@/store';
import { loadBundledData } from '@/data-loader';

// Mount UI
mountUpload(document.querySelector('#upload-zone')!);
mountFilters(document.querySelector('#filter-panel')!);

const heatmapEl = document.querySelector<HTMLElement>('#heatmap-chart')!;
const tableEl   = document.querySelector<HTMLElement>('#summary-table')!;
const drawerEl  = document.querySelector<HTMLElement>('#detail-drawer')!;

// Initial empty render
renderHeatmap(heatmapEl, []);
renderTable(tableEl, [], null);
renderSampleCard(drawerEl, null);

// Subscribe to state changes
subscribe(() => {
  const samples = getFiltered();
  const selected = store.selectedSample
    ? store.samples.find(s => s.sampleId === store.selectedSample) ?? null
    : null;
  renderHeatmap(heatmapEl, samples);
  renderTable(tableEl, samples, store.selectedSample);
  renderSampleCard(drawerEl, selected);
});

// Auto-load bundled data
loadBundledData().then(samples => {
  dispatch({ type: 'ADD_SAMPLES', samples });
});
```

- [ ] **Step 11.3: Run full integration test**

```bash
npm run dev
```
Open `http://localhost:5173` — all 20 samples load automatically, heatmap renders, table populates, filters work, clicking a sample opens the detail drawer with VAF and coverage charts.

- [ ] **Step 11.4: Run full test suite**

```bash
npm test
```
Expected: All tests PASS.

- [ ] **Step 11.5: Final commit**

```bash
git add src/data-loader.ts src/main.ts
git commit -m "feat: auto-load all 20 bundled TSV samples on startup"
```

---

## Self-Review

**Spec coverage check:**
- [x] File upload / drag-drop → Task 5
- [x] Cohort heatmap with click → Task 6
- [x] Summary table with sort/search/export → Task 7
- [x] Clinical therapy badge → Tasks 8, 7
- [x] VAF bar chart → Task 10
- [x] Read depth chart with low-reads warning → Task 10
- [x] Sample detail drawer → Task 8
- [x] Filter panel (cohort, AR-V7, minReads) → Task 9
- [x] Low-reads warning flag → parser `hasLowReads`, sampleCard
- [x] Auto-load bundled data → Task 11
- [x] Accessibility (focus states, aria labels, reduced motion) → CSS Task 4

**Type consistency check:** All functions use types from `src/types.ts`. `buildSampleSummary` and `SampleSummary` interface are consistent across parser/store/components. `TherapyRec` used uniformly.

**No placeholders:** All steps contain complete code.
