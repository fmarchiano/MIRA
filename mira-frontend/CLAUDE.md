# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Biological Context

This project visualizes results from **MIRA** (Mutation In RNA-seq Aligner), a tool that detects androgen receptor (AR) variants directly from bulk RNA-seq FASTQ files without genome alignment.

Metastatic castration-resistant prostate cancer (mCRPC) tumors escape androgen deprivation therapy by altering the AR. MIRA detects five clinically actionable alterations:

| Target | Type | Clinical Implication |
|--------|------|----------------------|
| AR-V7 (exon3/CE3 junction) | Splice variant | Switch from ARPI to taxane |
| AR-V7 CE3 full | Splice variant | Confirms AR-V7 positivity |
| T878A | Point mutation (ACT→GCT) | Abiraterone resistance → darolutamide |
| L702H | Point mutation (CTC→CAC) | Abiraterone resistance → darolutamide |
| W742C | Point mutation (TGG→TGT) | Anti-androgen resistance |
| H875Y | Point mutation (CAT→TAT) | Anti-androgen resistance |

**Therapy decision logic:**
- AR-V7+ → Taxane
- T878A or L702H MUT → Darolutamide
- All WT → Continue ARPI

## Dataset

- **20 RNA-seq samples**: 10 WCDT (mCRPC biopsies, `DTB-XXX-BL-T-RNA`) + 10 TCGA-PRAD (primary tumor, `TCGA-XX-XXXX-XXX`)
- **MIRA results location**: `~/wcdt_download/results/` and `tables/` in this repo
- **Three output files per sample**:
  - `{sample}_AR.summary.tsv` — primary clinical output (MUT/WT/PRESENCE/ABSENCE per target)
  - `{sample}_AR.tsv` — full per-position pileup
  - `{sample}_AR.novel.tsv` — unexpected high-VAF variants

### summary.tsv format
```
sample          target                        type              call      total_reads  alt_reads  vaf
DTB-034-BL      AR_V7_exon3_CE3_junction     SPLICE_JUNCTION   PRESENCE  1122         .          .
DTB-034-BL      AR_T878A_region              SNP               WT        202          0          0.0000
DTB-034-BL      AR_L702H_region              SNP               MUT       198          87         0.4394
```
- `call`: `PRESENCE`/`ABSENCE` for splice targets; `MUT`/`WT` for SNP targets
- `vaf`: Variant Allele Frequency (0–1); `.` for splice targets (use `null`)
- Samples with < 10 `total_reads` are unreliable — flag with a warning

## Frontend Stack

- **Language**: TypeScript (vanilla, no framework — no React/Vue/Angular)
- **Build**: Vite
- **Charts**: Plotly.js
- **Styling**: plain CSS with CSS custom properties (no Tailwind, no CSS-in-JS)

## Development Commands

```bash
npm install          # install dependencies
npm run dev          # start Vite dev server
npm run build        # production build
npm run preview      # preview production build
```

## Project Structure

```
mira-dashboard/
├── index.html
├── vite.config.ts
├── tsconfig.json
├── src/
│   ├── main.ts           # entry point
│   ├── parser.ts         # TSV parsing
│   ├── store.ts          # app state (loaded samples, filters)
│   ├── types.ts          # TypeScript interfaces
│   ├── style.css
│   ├── charts/
│   │   ├── heatmap.ts    # cohort heatmap (Plotly Heatmap trace)
│   │   ├── vafBar.ts     # VAF bar chart per sample
│   │   ├── waterfall.ts  # oncoprint
│   │   └── coverage.ts   # read depth bar chart
│   └── components/
│       ├── table.ts      # summary table renderer
│       ├── sampleCard.ts # per-sample clinical card
│       └── filters.ts    # filter panel UI
├── data/                 # TSV files (loaded via file picker or drag-drop)
└── tables/               # sample data checked into repo
```

## TypeScript Interfaces

```typescript
// types.ts
export type Call = 'PRESENCE' | 'ABSENCE' | 'MUT' | 'WT';
export type TargetType = 'SPLICE_JUNCTION' | 'SNP';
export type Cohort = 'WCDT' | 'TCGA' | 'UNKNOWN';

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
  therapyRecommendation: 'Taxane' | 'Darolutamide' | 'Continue ARPI' | 'Uncertain';
  alterationCount: number;
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

## Key Parsing Logic

```typescript
export function parseSummaryTsv(raw: string): MiraRecord[] {
  const lines = raw.trim().split('\n');
  return lines.slice(1).map(line => {
    const cols = line.split('\t');
    return {
      sample: cols[0], target: cols[1], type: cols[2] as TargetType,
      call: cols[3] as Call, totalReads: parseInt(cols[4]) || 0,
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

export function inferTherapy(records: MiraRecord[]): SampleSummary['therapyRecommendation'] {
  const get = (target: string) => records.find(r => r.target.includes(target))?.call;
  if (get('V7_exon3') === 'PRESENCE') return 'Taxane';
  if (get('T878A') === 'MUT' || get('L702H') === 'MUT') return 'Darolutamide';
  if (records.every(r => r.call === 'WT' || r.call === 'ABSENCE')) return 'Continue ARPI';
  return 'Uncertain';
}
```

## Design Direction

- **Aesthetic**: clinical/precision-medicine — dark background, monospace accents, data-forward layout
- **Color palette**:
  - MUT/PRESENCE: `#e63946` (red)
  - WT/ABSENCE: `#457b9d` (steel blue)
  - Warning (low reads): `#f4a261` (amber)
  - Background: `#0d1117` / Surface: `#161b22` / Border: `#30363d` / Text: `#e6edf3`
- **Layout**: left sidebar (upload + filters) → main heatmap → right drawer (sample detail)
- No heavy animations — researchers want data

## Architecture Notes

- All TSV parsing happens in the browser — no backend, no server
- All state lives in a plain TypeScript object in `store.ts` (no Redux/Zustand)
- The dashboard must work via `vite dev` or by opening `index.html` directly
- Prioritize correctness of biological interpretation over visual polish
- When a call is ambiguous (low reads, borderline VAF), show raw data and let the researcher decide
