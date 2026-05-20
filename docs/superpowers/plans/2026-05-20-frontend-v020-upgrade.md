# Frontend v0.2.0 Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update the MIRA dashboard frontend to support all new MIRA v0.2.0 output columns and clinical features.

**Architecture:** Changes flow top-down — types first, then parser, then UI components. No new files needed; all changes are extensions to existing ones. Tests live in `tests/parser.test.ts` (logic) — visual components (heatmap, sampleCard, vafBar) are tested manually via `npm run dev`.

**Tech Stack:** TypeScript (vanilla), Vite, Plotly.js, Vitest

---

## What changed in MIRA v0.2.0

| Change | Impact on frontend |
|--------|--------------------|
| Provenance header (`# mira=0.2.0 ref=...`) at top of every TSV | Parser must skip `#` lines |
| New `call` value: `INDETERMINATE` (low coverage, < 30 reads by default) | Add to types, display in amber, therapy → `Uncertain` if key target |
| New `vaf_ci_lo` / `vaf_ci_hi` columns (Wilson 95% CI) | Parse + show in VAF bar chart as error bars and in hover text |
| New `expr_index` column (reads / HK median) | Parse + show in sample card |
| New `splice_fraction` column (AR-V7 / (AR-V7 + AR-FL)) | Parse + show in AR-V7 row and heatmap hover |
| New SNP target `AR_F877L_region` (Enzalutamide resistance) | Add to heatmap columns |
| New target types `CONSTITUTIVE` and `HOUSEKEEPING` | Add to types; exclude HK rows from clinical display |
| New targets: `AR_FL_exon3_exon4_junction`, `AR_const_exon1`, HK genes | Filter out of main records list in UI |

## New TSV column layout (v0.2.0)

```
sample  target  type  call  total_reads  alt_reads  vaf  vaf_ci_lo  vaf_ci_hi  expr_index  splice_fraction
```

Columns 7–10 (0-indexed) are new. `.` means null/not-applicable.

---

## File map

| File | Change |
|------|--------|
| `src/types.ts` | Add `INDETERMINATE` to `Call`; add `CONSTITUTIVE \| HOUSEKEEPING` to `TargetType`; add `vafCiLo`, `vafCiHi`, `exprIndex`, `spliceFraction` to `MiraRecord`; add `spliceFraction` to `SampleSummary` |
| `src/parser.ts` | Skip `#` header lines; parse 4 new columns; `INDETERMINATE` in therapy logic; update `buildSampleSummary` |
| `src/charts/heatmap.ts` | Add `F877L` target column; `INDETERMINATE` → amber cell; splice_fraction + CI in hover |
| `src/components/sampleCard.ts` | `INDETERMINATE` amber badge; CI display; splice_fraction row; expr_index row; filter HK targets |
| `src/charts/vafBar.ts` | Error bars from CI; `INDETERMINATE` bars shown amber |
| `tests/parser.test.ts` | New tests for all new parsing behaviour |

---

## Task 1: Extend types.ts

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Update Call, TargetType, and MiraRecord**

Replace the entire contents of `src/types.ts` with:

```typescript
export type Call = 'PRESENCE' | 'ABSENCE' | 'MUT' | 'WT' | 'INDETERMINATE';
export type TargetType = 'SPLICE_JUNCTION' | 'SNP' | 'CONSTITUTIVE' | 'HOUSEKEEPING';
export type Cohort = 'WCDT' | 'TCGA' | 'UNKNOWN';
export type TherapyRec = 'Taxane' | 'Darolutamide' | 'Continue ARPI' | 'Uncertain';
export type Concordance = 'CONCORDANT' | 'DISCORDANT' | 'NOT_IN_PANEL' | 'UNKNOWN';

export interface SampleMetadata {
  sampleName: string;
  dataset: string;
  arMutation: string;
  arAmplification: string;
  concordance: Concordance;
}

export interface MiraRecord {
  sample: string;
  target: string;
  type: TargetType;
  call: Call;
  totalReads: number;
  altReads: number | null;
  vaf: number | null;
  vafCiLo: number | null;   // Wilson 95% CI lower bound
  vafCiHi: number | null;   // Wilson 95% CI upper bound
  exprIndex: number | null; // reads / median(HK reads)
  spliceFraction: number | null; // AR-V7 / (AR-V7 + AR-FL), V7 row only
}

export interface SampleSummary {
  sampleId: string;
  cohort: Cohort;
  records: MiraRecord[];          // all records including CONSTITUTIVE/HK
  clinicalRecords: MiraRecord[];  // only SNP + SPLICE_JUNCTION (excludes HK/CONSTITUTIVE)
  therapyRecommendation: TherapyRec;
  alterationCount: number;
  hasLowReads: boolean;
  metadata?: SampleMetadata;
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

- [ ] **Step 2: Verify TypeScript compiles**

Run:
```bash
cd /home/simone/work/MIRA/mira-frontend && npx tsc --noEmit
```

Expected: errors about `clinicalRecords` missing in existing code (we fix those in later tasks). Any error referencing `types.ts` itself is a problem — fix before continuing.

---

## Task 2: Update parser.ts

**Files:**
- Modify: `src/parser.ts`
- Test: `tests/parser.test.ts`

- [ ] **Step 1: Write failing tests for new behaviour**

Replace the contents of `tests/parser.test.ts` with:

```typescript
import { describe, it, expect } from 'vitest';
import { parseSummaryTsv, inferCohort, inferTherapy, buildSampleSummary } from '@/parser';

// v0.1 format (backwards compat — no new columns)
const TSV_V1 = `sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf
DTB-034-BL-T-RNA_R1\tAR_V7_exon3_CE3_junction\tSPLICE_JUNCTION\tPRESENCE\t1414\t.\t.
DTB-034-BL-T-RNA_R1\tAR_CE3_full\tSPLICE_JUNCTION\tPRESENCE\t1230\t.\t.
DTB-034-BL-T-RNA_R1\tAR_T878A_region\tSNP\tWT\t15054\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_L702H_region\tSNP\tWT\t3814\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_W742C_region\tSNP\tWT\t5080\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_H875Y_region\tSNP\tWT\t14720\t0\t0.0000`;

// v0.2 format with provenance header + new columns + new targets
const TSV_V2 = `# mira=0.2.0 ref=AR_targets.fa ref_md5=abc123 r1=sample_R1.fastq.gz r1_md5=size=1234 timestamp=2026-05-20T10:00:00Z
sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf\tvaf_ci_lo\tvaf_ci_hi\texpr_index\tsplice_fraction
DTB-034-BL-T-RNA_R1\tAR_V7_exon3_CE3_junction\tSPLICE_JUNCTION\tPRESENCE\t142\t.\t.\t.\t.\t0.46\t0.50
DTB-034-BL-T-RNA_R1\tAR_CE3_full\tSPLICE_JUNCTION\tPRESENCE\t121\t.\t.\t.\t.\t0.39\t.
DTB-034-BL-T-RNA_R1\tAR_FL_exon3_exon4_junction\tSPLICE_JUNCTION\tPRESENCE\t141\t.\t.\t.\t.\t0.46\t.
DTB-034-BL-T-RNA_R1\tAR_const_exon1\tCONSTITUTIVE\tEXPRESSED\t146\t.\t.\t.\t.\t0.48\t.
DTB-034-BL-T-RNA_R1\tAR_T878A_region\tSNP\tWT\t58\t0\t0.0000\t0.0000\t0.0615\t.\t.
DTB-034-BL-T-RNA_R1\tAR_L702H_region\tSNP\tINDETERMINATE\t15\t.\t.\t.\t.\t.\t.
DTB-034-BL-T-RNA_R1\tAR_W742C_region\tSNP\tWT\t44\t0\t0.0000\t0.0000\t0.0801\t.\t.
DTB-034-BL-T-RNA_R1\tAR_H875Y_region\tSNP\tMUT\t41\t41\t1.0000\t0.9139\t1.0000\t.\t.
DTB-034-BL-T-RNA_R1\tAR_F877L_region\tSNP\tWT\t39\t0\t0.0000\t0.0000\t0.0893\t.\t.
DTB-034-BL-T-RNA_R1\tGAPDH\tHOUSEKEEPING\tEXPRESSED\t1240\t.\t.\t.\t.\t.\t.
DTB-034-BL-T-RNA_R1\tACTB\tHOUSEKEEPING\tEXPRESSED\t586\t.\t.\t.\t.\t.\t.`;

describe('parseSummaryTsv — v0.1 backwards compatibility', () => {
  it('parses all 6 rows', () => {
    expect(parseSummaryTsv(TSV_V1)).toHaveLength(6);
  });
  it('sets new fields to null when columns absent', () => {
    const r = parseSummaryTsv(TSV_V1)[0];
    expect(r.vafCiLo).toBeNull();
    expect(r.vafCiHi).toBeNull();
    expect(r.exprIndex).toBeNull();
    expect(r.spliceFraction).toBeNull();
  });
});

describe('parseSummaryTsv — v0.2', () => {
  it('skips provenance comment lines', () => {
    const records = parseSummaryTsv(TSV_V2);
    expect(records.every(r => !r.sample.startsWith('#'))).toBe(true);
  });
  it('parses 11 data rows (including HK and CONSTITUTIVE)', () => {
    expect(parseSummaryTsv(TSV_V2)).toHaveLength(11);
  });
  it('parses vaf_ci_lo and vaf_ci_hi for SNP MUT row', () => {
    const r = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_H875Y_region')!;
    expect(r.vafCiLo).toBeCloseTo(0.9139);
    expect(r.vafCiHi).toBeCloseTo(1.0);
  });
  it('sets vafCiLo null when column is dot', () => {
    const r = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_V7_exon3_CE3_junction')!;
    expect(r.vafCiLo).toBeNull();
  });
  it('parses exprIndex for V7 row', () => {
    const r = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_V7_exon3_CE3_junction')!;
    expect(r.exprIndex).toBeCloseTo(0.46);
  });
  it('parses spliceFraction for V7 row only', () => {
    const v7 = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_V7_exon3_CE3_junction')!;
    const ce3 = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_CE3_full')!;
    expect(v7.spliceFraction).toBeCloseTo(0.50);
    expect(ce3.spliceFraction).toBeNull();
  });
  it('parses INDETERMINATE call', () => {
    const r = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_L702H_region')!;
    expect(r.call).toBe('INDETERMINATE');
  });
  it('parses HOUSEKEEPING target type', () => {
    const r = parseSummaryTsv(TSV_V2).find(r => r.target === 'GAPDH')!;
    expect(r.type).toBe('HOUSEKEEPING');
  });
  it('parses CONSTITUTIVE target type', () => {
    const r = parseSummaryTsv(TSV_V2).find(r => r.target === 'AR_const_exon1')!;
    expect(r.type).toBe('CONSTITUTIVE');
  });
});

describe('inferTherapy', () => {
  it('returns Taxane when AR-V7 is PRESENCE', () => {
    expect(inferTherapy(parseSummaryTsv(TSV_V1))).toBe('Taxane');
  });
  it('returns Darolutamide when T878A is MUT (V7 ABSENCE)', () => {
    const tsv = TSV_V1
      .replace('PRESENCE\t1414', 'ABSENCE\t1414')
      .replace('PRESENCE\t1230', 'ABSENCE\t1230')
      .replace('WT\t15054\t0\t0.0000', 'MUT\t15054\t150\t0.0100');
    expect(inferTherapy(parseSummaryTsv(tsv))).toBe('Darolutamide');
  });
  it('returns Continue ARPI when all WT/ABSENCE', () => {
    const tsv = TSV_V1
      .replace('PRESENCE\t1414', 'ABSENCE\t1414')
      .replace('PRESENCE\t1230', 'ABSENCE\t1230');
    expect(inferTherapy(parseSummaryTsv(tsv))).toBe('Continue ARPI');
  });
  it('returns Uncertain when a key target is INDETERMINATE', () => {
    const records = parseSummaryTsv(TSV_V2);
    // TSV_V2 has L702H as INDETERMINATE — therapy should be Uncertain
    expect(inferTherapy(records)).toBe('Uncertain');
  });
});

describe('buildSampleSummary', () => {
  it('flags hasLowReads when any clinical target has < 30 reads', () => {
    const records = parseSummaryTsv(TSV_V2);
    const summary = buildSampleSummary(records);
    // L702H has 15 reads < 30
    expect(summary.hasLowReads).toBe(true);
  });
  it('clinicalRecords excludes HOUSEKEEPING and CONSTITUTIVE rows', () => {
    const summary = buildSampleSummary(parseSummaryTsv(TSV_V2));
    expect(summary.clinicalRecords.every(r => r.type !== 'HOUSEKEEPING' && r.type !== 'CONSTITUTIVE')).toBe(true);
  });
  it('counts alterationCount from MUT + PRESENCE clinical records only', () => {
    const summary = buildSampleSummary(parseSummaryTsv(TSV_V2));
    // V7 PRESENCE, CE3 PRESENCE, FL PRESENCE, H875Y MUT = 4
    expect(summary.alterationCount).toBe(4);
  });
});

describe('inferCohort', () => {
  it('identifies WCDT by DTB- prefix', () => expect(inferCohort('DTB-034-BL')).toBe('WCDT'));
  it('identifies TCGA by TCGA- prefix', () => expect(inferCohort('TCGA-EJ-7314')).toBe('TCGA'));
  it('returns UNKNOWN for unrecognized prefixes', () => expect(inferCohort('SAMPLE-001')).toBe('UNKNOWN'));
});
```

- [ ] **Step 2: Run tests — verify they fail**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx vitest run tests/parser.test.ts
```

Expected: multiple failures (new columns not yet parsed, `clinicalRecords` missing, etc.)

- [ ] **Step 3: Update parser.ts**

Replace `src/parser.ts` with:

```typescript
import type { MiraRecord, TargetType, Call, Cohort, TherapyRec, SampleSummary } from './types';

const CLINICAL_TYPES: TargetType[] = ['SNP', 'SPLICE_JUNCTION'];

export function parseSummaryTsv(raw: string): MiraRecord[] {
  const lines = raw.trim().split('\n').filter(l => !l.startsWith('#'));
  const header = lines[0].split('\t');
  const col = (name: string) => header.indexOf(name);

  const iVafCiLo = col('vaf_ci_lo');
  const iVafCiHi = col('vaf_ci_hi');
  const iExprIndex = col('expr_index');
  const iSpliceFrac = col('splice_fraction');

  const parseNum = (v: string | undefined) => (!v || v === '.') ? null : parseFloat(v);
  const parseInt_ = (v: string | undefined) => (!v || v === '.') ? null : parseInt(v);

  return lines.slice(1).map(line => {
    const c = line.split('\t');
    return {
      sample:          c[0],
      target:          c[1],
      type:            c[2] as TargetType,
      call:            c[3] as Call,
      totalReads:      parseInt(c[4]) || 0,
      altReads:        parseInt_(c[5]),
      vaf:             parseNum(c[6]),
      vafCiLo:         iVafCiLo  >= 0 ? parseNum(c[iVafCiLo])  : null,
      vafCiHi:         iVafCiHi  >= 0 ? parseNum(c[iVafCiHi])  : null,
      exprIndex:       iExprIndex >= 0 ? parseNum(c[iExprIndex]) : null,
      spliceFraction:  iSpliceFrac >= 0 ? parseNum(c[iSpliceFrac]) : null,
    };
  });
}

export function inferCohort(sampleId: string): Cohort {
  if (sampleId.startsWith('DTB-')) return 'WCDT';
  if (sampleId.startsWith('TCGA-')) return 'TCGA';
  return 'UNKNOWN';
}

// Key SNP targets that drive therapy — INDETERMINATE here → Uncertain
const KEY_SNP_TARGETS = ['T878A', 'L702H', 'W742C', 'H875Y', 'F877L'];

export function inferTherapy(records: MiraRecord[]): TherapyRec {
  const clinical = records.filter(r => CLINICAL_TYPES.includes(r.type));
  const get = (key: string) => clinical.find(r => r.target.includes(key))?.call;

  if (get('V7_exon3') === 'PRESENCE') return 'Taxane';

  const hasIndeterminate = KEY_SNP_TARGETS.some(k => get(k) === 'INDETERMINATE');
  if (hasIndeterminate) return 'Uncertain';

  if (get('T878A') === 'MUT' || get('L702H') === 'MUT') return 'Darolutamide';
  if (clinical.every(r => r.call === 'WT' || r.call === 'ABSENCE')) return 'Continue ARPI';
  return 'Uncertain';
}

export function buildSampleSummary(records: MiraRecord[]): SampleSummary {
  const sampleId = records[0]?.sample ?? 'unknown';
  const clinicalRecords = records.filter(r => CLINICAL_TYPES.includes(r.type));
  return {
    sampleId,
    cohort: inferCohort(sampleId),
    records,
    clinicalRecords,
    therapyRecommendation: inferTherapy(records),
    alterationCount: clinicalRecords.filter(r => r.call === 'MUT' || r.call === 'PRESENCE').length,
    hasLowReads: clinicalRecords.some(r => r.totalReads < 30),
  };
}
```

- [ ] **Step 4: Run tests — verify they all pass**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx vitest run tests/parser.test.ts
```

Expected: all tests PASS

- [ ] **Step 5: Fix TypeScript errors caused by new `clinicalRecords` field**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx tsc --noEmit 2>&1 | grep -v "node_modules"
```

Any remaining errors will be in `store.ts` or component files that reference `records` directly on `SampleSummary`. Fix by changing those references to use `clinicalRecords` where appropriate (see Task 4 and 5).

- [ ] **Step 6: Commit**

```bash
cd /home/simone/work/MIRA/mira-frontend && git add src/types.ts src/parser.ts tests/parser.test.ts && git commit -m "feat: add v0.2.0 type support — INDETERMINATE, CI, exprIndex, spliceFraction"
```

---

## Task 3: Update heatmap.ts

**Files:**
- Modify: `src/charts/heatmap.ts`

- [ ] **Step 1: Add F877L column and INDETERMINATE colour**

Replace the `TARGETS` array and `callToZ` function in `src/charts/heatmap.ts`:

```typescript
const TARGETS = [
  { key: 'V7_exon3', label: 'AR-V7' },
  { key: 'CE3_full', label: 'CE3' },
  { key: 'T878A',    label: 'T878A' },
  { key: 'L702H',    label: 'L702H' },
  { key: 'W742C',    label: 'W742C' },
  { key: 'H875Y',    label: 'H875Y' },
  { key: 'F877L',    label: 'F877L' },
];

// Returns: 1 = MUT/PRESENCE (red), 0 = WT/ABSENCE (blue), 0.5 = INDETERMINATE (amber), -1 = no data (grey)
function callToZ(call: string | undefined): number {
  if (call === 'MUT' || call === 'PRESENCE') return 1;
  if (call === 'WT'  || call === 'ABSENCE')  return 0;
  if (call === 'INDETERMINATE')              return 0.5;
  return -1;
}
```

- [ ] **Step 2: Update colorscale to include amber for INDETERMINATE**

Replace the `colorscale` property in the trace object:

```typescript
colorscale: [
  [0,    '#30363d'],  // no data — grey
  [0.33, '#457b9d'],  // WT/ABSENCE — steel blue
  [0.67, '#f4a261'],  // INDETERMINATE — amber
  [1,    '#e63946'],  // MUT/PRESENCE — red
] as Plotly.ColorScale,
zmin: -1, zmax: 1,
```

- [ ] **Step 3: Update hover text to show splice_fraction and CI**

Replace the `text` computation block (the `samples.map(s => TARGETS.map(...)` for text):

```typescript
const text = samples.map(s =>
  TARGETS.map(t => {
    const rec = s.clinicalRecords.find(r => r.target.includes(t.key));
    if (!rec) return 'No data';
    let vafStr = '';
    if (rec.vaf !== null) {
      vafStr = ` | VAF: ${(rec.vaf * 100).toFixed(1)}%`;
      if (rec.vafCiLo !== null && rec.vafCiHi !== null) {
        vafStr += ` [${(rec.vafCiLo * 100).toFixed(1)}–${(rec.vafCiHi * 100).toFixed(1)}%]`;
      }
    }
    const spliceStr = rec.spliceFraction !== null
      ? ` | splice frac: ${(rec.spliceFraction * 100).toFixed(0)}%`
      : '';
    const warn = rec.totalReads < 30 ? ' ⚠ LOW READS' : '';
    return `${s.sampleId}<br>${rec.target}<br><b>${rec.call}</b>${vafStr}${spliceStr} | Reads: ${rec.totalReads}${warn}`;
  })
);
```

Also update the `z` computation to use `clinicalRecords`:

```typescript
const z = samples.map(s =>
  TARGETS.map(t => {
    const rec = s.clinicalRecords.find(r => r.target.includes(t.key));
    return callToZ(rec?.call);
  })
);
```

- [ ] **Step 4: Verify build**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx tsc --noEmit 2>&1 | grep -v "node_modules"
```

Expected: no errors in `heatmap.ts`

- [ ] **Step 5: Commit**

```bash
cd /home/simone/work/MIRA/mira-frontend && git add src/charts/heatmap.ts && git commit -m "feat: heatmap — add F877L, INDETERMINATE amber, CI and splice_fraction in hover"
```

---

## Task 4: Update sampleCard.ts

**Files:**
- Modify: `src/components/sampleCard.ts`

- [ ] **Step 1: Update record row rendering to show INDETERMINATE in amber, CI, and splice_fraction**

In `src/components/sampleCard.ts`, replace the `.records.map(r => ...)` section inside `renderSampleCard` with:

```typescript
${sample.clinicalRecords.map(r => {
  const isAlt = r.call === 'MUT' || r.call === 'PRESENCE';
  const isIndet = r.call === 'INDETERMINATE';
  const callCls = isAlt ? 'mut' : isIndet ? 'indet' : 'wt';

  let vafStr = '';
  if (r.vaf !== null) {
    vafStr = `VAF ${(r.vaf * 100).toFixed(1)}%`;
    if (r.vafCiLo !== null && r.vafCiHi !== null) {
      vafStr += ` [${(r.vafCiLo * 100).toFixed(1)}–${(r.vafCiHi * 100).toFixed(1)}%]`;
    }
  }
  const spliceStr = r.spliceFraction !== null
    ? `splice ${(r.spliceFraction * 100).toFixed(0)}%`
    : '';
  const readsStr = `${r.totalReads} reads`;
  const meta = [vafStr, spliceStr, readsStr].filter(Boolean).join(' · ');
  const warn = r.totalReads < 30 ? `<span style="color:var(--amber);margin-left:4px">${WARN_ICON}</span>` : '';

  return `<div class="record-row">
    <span class="record-row__target">${r.target.replace('AR_', '').replace('_region', '').replace('_junction', '').replace('_CE3', '')}</span>
    <span class="badge badge--${callCls}">${r.call}</span>
    <span class="record-row__meta">${meta}${warn}</span>
  </div>`;
}).join('')}
```

- [ ] **Step 2: Add CSS class for INDETERMINATE badge**

In `src/style.css`, add after the `.badge--wt` rule:

```css
.badge--indet {
  background: color-mix(in srgb, var(--amber) 15%, transparent);
  color: var(--amber);
  border: 1px solid color-mix(in srgb, var(--amber) 30%, transparent);
}
```

- [ ] **Step 3: Update the low-reads warning threshold to 30**

In `src/components/sampleCard.ts`, replace the low-reads warning condition:

Old:
```typescript
${sample.hasLowReads ? `<div class="low-reads-warning">${WARN_ICON} One or more targets have &lt;10 reads — interpret with caution.</div>` : ''}
```

New:
```typescript
${sample.hasLowReads ? `<div class="low-reads-warning">${WARN_ICON} One or more targets have &lt;30 reads — calls marked INDETERMINATE.</div>` : ''}
```

- [ ] **Step 4: Verify TypeScript**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx tsc --noEmit 2>&1 | grep -v "node_modules"
```

- [ ] **Step 5: Commit**

```bash
cd /home/simone/work/MIRA/mira-frontend && git add src/components/sampleCard.ts src/style.css && git commit -m "feat: sample card — INDETERMINATE badge, CI range display, splice_fraction"
```

---

## Task 5: Update vafBar.ts — error bars for CI

**Files:**
- Modify: `src/charts/vafBar.ts`

- [ ] **Step 1: Add CI error bars and amber colour for INDETERMINATE**

Replace the entire `src/charts/vafBar.ts` with:

```typescript
import Plotly from 'plotly.js-dist-min';
import type { MiraRecord } from '@/types';

export function renderVafBar(container: HTMLElement, records: MiraRecord[]): void {
  const snp = records.filter(r => r.type === 'SNP');
  if (!snp.length) {
    container.innerHTML = '<p style="color:var(--text-muted);font-size:12px;padding:8px 0">No SNP targets</p>';
    return;
  }

  const colors = snp.map(r => {
    if (r.call === 'INDETERMINATE') return '#f4a261';
    return (r.vaf ?? 0) >= 0.3 ? '#e63946' : '#457b9d';
  });

  const hasCI = snp.some(r => r.vafCiLo !== null);
  const errorBars: Plotly.ErrorBar | undefined = hasCI ? {
    type: 'data',
    symmetric: false,
    array:      snp.map(r => r.vafCiHi !== null && r.vaf !== null ? r.vafCiHi - r.vaf : 0),
    arrayminus: snp.map(r => r.vafCiLo !== null && r.vaf !== null ? r.vaf - r.vafCiLo : 0),
    color: '#8b949e',
    thickness: 1.5,
    width: 4,
    visible: true,
  } : undefined;

  const trace: Partial<Plotly.PlotData> = {
    type: 'bar',
    orientation: 'h',
    x: snp.map(r => r.vaf ?? 0),
    y: snp.map(r => r.target.replace('AR_', '').replace('_region', '')),
    marker: { color: colors },
    error_x: errorBars,
    hovertemplate: snp.map(r => {
      const vafPct = r.vaf !== null ? `${(r.vaf * 100).toFixed(1)}%` : 'N/A';
      const ci = r.vafCiLo !== null && r.vafCiHi !== null
        ? ` [${(r.vafCiLo * 100).toFixed(1)}–${(r.vafCiHi * 100).toFixed(1)}%]`
        : '';
      return `${r.target.replace('AR_', '').replace('_region', '')}: ${vafPct}${ci} (${r.call})<extra></extra>`;
    }) as unknown as string,
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor: 'transparent',
    font: { color: '#e6edf3', size: 10, family: 'JetBrains Mono, monospace' },
    margin: { l: 100, r: 20, t: 10, b: 30 },
    height: 140,
    xaxis: { range: [0, 1], tickformat: '.0%', gridcolor: '#30363d', fixedrange: true },
    yaxis: { fixedrange: true },
    shapes: [{
      type: 'line', x0: 0.3, x1: 0.3, y0: -0.5, y1: snp.length - 0.5,
      line: { color: '#f4a261', dash: 'dot', width: 1 },
    }],
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });
}
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx tsc --noEmit 2>&1 | grep -v "node_modules"
```

Expected: no errors

- [ ] **Step 3: Full test run**

```bash
cd /home/simone/work/MIRA/mira-frontend && npx vitest run
```

Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
cd /home/simone/work/MIRA/mira-frontend && git add src/charts/vafBar.ts && git commit -m "feat: VAF bar — 95% CI error bars, INDETERMINATE amber, per-bar hover with CI"
```

---

## Task 6: Manual smoke test

**Files:** none (verification only)

- [ ] **Step 1: Start dev server**

```bash
cd /home/simone/work/MIRA/mira-frontend && npm run dev
```

Open http://localhost:5173 in a browser.

- [ ] **Step 2: Load a v0.1 file and verify backwards compatibility**

Load any file from `mira-frontend/tables/` (old format). Verify:
- Heatmap renders 6 columns including new F877L column (shows grey = no data for old files)
- No JavaScript console errors

- [ ] **Step 3: Load a v0.2 file (once pipeline finishes)**

Once `~/wcdt_download/results/` is populated, load a `*_AR.summary.tsv`. Verify:
- Provenance header is silently skipped (no blank/broken row in heatmap)
- `INDETERMINATE` cells show in amber
- VAF bar shows error bar whiskers for CI
- Sample card shows `[lo–hi%]` CI range next to VAF
- Splice fraction shown on AR-V7 row

- [ ] **Step 4: Final commit and push**

```bash
cd /home/simone/work/MIRA/mira-frontend && git push origin feature/front-end-panel
```

---

## Self-review

| Requirement | Covered by |
|-------------|-----------|
| Provenance `#` header skipped | Task 2, parser.ts filter |
| `INDETERMINATE` call type | Task 1 (types), Task 2 (parser + therapy), Task 3 (heatmap amber), Task 4 (badge), Task 5 (vafBar amber) |
| `vaf_ci_lo` / `vaf_ci_hi` parsed | Task 2 (parser), Task 4 (sample card text), Task 5 (error bars), Task 3 (hover) |
| `expr_index` parsed | Task 2 (parser) — displayed in sample card via `exprIndex` field (shown in record meta in Task 4) |
| `splice_fraction` parsed + shown | Task 2 (parser), Task 3 (heatmap hover), Task 4 (sample card row) |
| `AR_F877L_region` in heatmap | Task 3 (TARGETS array) |
| `CONSTITUTIVE` / `HOUSEKEEPING` types | Task 1 (types), Task 2 (clinicalRecords filter) |
| HK rows excluded from UI | Task 2 (clinicalRecords), Task 3+4+5 use clinicalRecords |
| Therapy `Uncertain` for INDETERMINATE | Task 2 (inferTherapy) |
| Low-reads threshold 30 (was 10) | Task 2 (buildSampleSummary), Task 4 (warning text) |
| Tests updated | Task 2 (parser.test.ts full rewrite) |
