# MIRA Dashboard

Browser-based visualization of MIRA pipeline output. No backend — all TSV parsing runs client-side.

## Setup

```bash
npm install
npm run dev      # http://localhost:5173
npm run build    # production → dist/
```

## Pipeline output files

Drop MIRA output files into `mira-frontend/tables/`:

```
tables/
  {sample}_AR.summary.tsv   ← primary input (one row per target)
  {sample}_AR.tsv            ← full pileup (optional)
  {sample}_AR.novel.tsv      ← novel variants (optional)
```

The dashboard auto-loads every `*.summary.tsv` found in `tables/` at startup. The `_AR.tsv` and `_AR.novel.tsv` files are loaded on demand when a sample is selected.

## Expected summary.tsv columns

```
sample  target  type  call  total_reads  alt_reads  vaf
```

- `call`: `PRESENCE`/`ABSENCE` (splice) or `MUT`/`WT` (SNP)
- `vaf`: float 0–1, or `.` for splice targets
- Samples with `total_reads < 10` are flagged as unreliable
