# MIRA

**Mutation In RNA-seq Aligner** — alignment-free androgen receptor variant detection from bulk RNA-seq FASTQ.

MIRA scans FASTQ files directly for AR splice variants (AR-V7) and resistance mutations (T878A, L702H, W742C, H875Y, F877L) without reference genome alignment. It extracts AR-matching reads via k-mer indexing, aligns them with Smith-Waterman, deduplicates PCR duplicates, and calls variants from pileup — producing three output tiers plus expression-normalized summary per sample.

---

## Clinical motivation

AR status determines treatment in metastatic castration-resistant prostate cancer (mCRPC):

| Finding | Implication |
|---------|-------------|
| AR-V7+ | Switch to taxane (enzalutamide/abiraterone: 0% PSA response) |
| T878A / L702H | Abiraterone resistance — switch to darolutamide |
| H875Y / F877L | Enzalutamide resistance |
| AR-V7− / no LBD mutations | Continue ARPI |

MIRA runs on existing bulk RNA-seq data — no dedicated assay required.

---

## Installation

### Pre-compiled binary (Linux x86_64, static — no dependencies)

```bash
curl -LO https://github.com/fmarchiano/MIRA/releases/latest/download/mira
chmod +x mira
./mira --help
```

### Reference files

```bash
curl -LO https://github.com/fmarchiano/MIRA/releases/latest/download/AR_targets.fa
curl -LO https://github.com/fmarchiano/MIRA/releases/latest/download/housekeeping.fa
```

### Build from source

**Requirements:** Rust ≥ 1.75, Cargo

```bash
git clone https://github.com/fmarchiano/MIRA.git
cd MIRA/mira
cargo build --release
# binary: ./target/release/mira
```

---

## Quick start

```bash
# Paired-end RNA-seq — AR targets only
mira \
  -i sample_R1.fastq sample_R2.fastq \
  -r reference/AR_targets.fa \
  -o results/sample_AR.tsv \
  -t 4

# With housekeeping normalization (recommended)
mira \
  -i sample_R1.fastq sample_R2.fastq \
  -r reference/AR_targets.fa \
  --housekeeping reference/housekeeping.fa \
  -o results/sample_AR.tsv \
  -t 4

# Single-end
mira \
  -i sample.fastq \
  -r reference/AR_targets.fa \
  -o results/sample_AR.tsv
```

Three output files are written automatically:

| File | Content |
|------|---------|
| `sample_AR.tsv` | Granular pileup — one row per variant position per target |
| `sample_AR.summary.tsv` | One row per target: PRESENCE/ABSENCE/INDETERMINATE or MUT/WT/INDETERMINATE, with `expr_index` |
| `sample_AR.novel.tsv` | High-VAF variants not matching any expected mutation |

---

## Reference files

### `reference/AR_targets.fa`

Nine 150–160 bp sequences covering clinically actionable AR sites:

| Target | Type | Variant / Role | Source |
|--------|------|----------------|--------|
| AR_V7_exon3_CE3_junction | SPLICE_JUNCTION | AR-V7 detection | NM_000044.6 + FJ235916.1 |
| AR_CE3_full | SPLICE_JUNCTION | AR-V7 (CE3 full) | FJ235916.1 |
| AR_T878A_region | SNP | T878A (ACT→GCT) | NM_000044.6 |
| AR_L702H_region | SNP | L702H (CTC→CAC) | NM_000044.6 |
| AR_W742C_region | SNP | W742C (TGG→TGT) | NM_000044.6 |
| AR_H875Y_region | SNP | H875Y (CAT→TAT) | NM_000044.6 |
| AR_F877L_region | SNP | F877L (TTC→CTC) | NM_000044.6 |
| AR_FL_exon3_exon4_junction | SPLICE_JUNCTION | AR-FL constitutive splice (V7 denominator) | NM_000044.6 |
| AR_const_exon1 | CONSTITUTIVE | AR amplification proxy (NTD exon 1) | NM_000044.6 |

Additional targets (ARv567es, AR-V3, AR-V9) can be added by appending sequences to the FASTA. Each target should be ~150 bp centered on the variant site (75 bp flanking per side, k=31).

**Reference codon validation:** at startup MIRA checks every SNP target's `_ref=` codon in the FASTA header against the actual bytes at `offset=`. A mismatch aborts with a clear error, catching annotation typos before a run produces silently wrong calls.

### `reference/housekeeping.fa`

Five 153 bp housekeeping gene windows for expression normalization:

| Gene | Accession | CDS window |
|------|-----------|------------|
| GAPDH | NM_002046.7 | pos 337–489 |
| ACTB | NM_001101.5 | pos 376–528 |
| HPRT1 | NM_000194.3 | pos 220–372 |
| B2M | NM_004048.4 | pos 121–273 |
| TBP | NM_003194.5 | pos 340–492 |

When passed via `--housekeeping`, MIRA adds these targets to the same k-mer index and emits an `expr_index` column in the summary (`target_reads / median(HK_reads)`). This normalizes AR target coverage to constitutive expression level.

---

## Options

```
Usage: mira [OPTIONS] --input <INPUT>... --reference <REFERENCE> --output <OUTPUT>

Options:
  -i, --input <INPUT>...           Input FASTQ file(s): one (single-end) or two (R1 R2, paired-end)
  -r, --reference <REFERENCE>      Reference target sequences (FASTA)
  -o, --output <OUTPUT>            Output TSV file (summary and novel TSVs auto-derived)
  -k, --kmer-size <K>              K-mer size [default: 31]
  -m, --max-mismatches <M>         Max Hamming mismatches for k-mer lookup [default: 2]
      --min-reads <N>              Min supporting reads to call a variant [default: 10]
      --min-mean-qual <Q>          Min mean Phred quality to keep a read [default: 20]
      --min-base-qual <Q>          Min per-base Phred quality in pileup [default: 20]
      --library-type <TYPE>        Library strandedness: unstranded | forward | reverse [default: unstranded]
      --min-mut-vaf <VAF>          Min VAF to call a known SNP as MUT in summary [default: 0.30]
      --novel-min-vaf <VAF>        Min VAF to report in novel variants output [default: 0.10]
      --min-coverage <N>           Min reads to call WT/ABSENCE; below → INDETERMINATE [default: 30]
      --housekeeping <FASTA>       Housekeeping gene FASTA for expression normalization (optional)
      --no-dedup                   Skip PCR duplicate removal by read sequence
      --save-extracted <PATH>      Write extracted AR reads to FASTQ (optional)
  -t, --threads <T>                Threads [default: logical CPUs]
  -h, --help                       Print help
```

---

## Output format

### `<sample>.summary.tsv`

One row per target. Primary output for clinical interpretation.

```
sample           target                        type              call       total_reads  alt_reads  vaf     vaf_ci_lo  vaf_ci_hi  splice_fraction  expr_index
SRR26125073_1    AR_V7_exon3_CE3_junction     SPLICE_JUNCTION   PRESENCE   102          .          .       .          .          0.3923           0.3682
SRR26125073_1    AR_FL_exon3_exon4_junction   SPLICE_JUNCTION   PRESENCE   158          .          .       .          .          .                0.5704
SRR26125073_1    AR_H875Y_region              SNP               MUT        60           40         1.0000  0.9124     1.0000     .                0.2166
SRR26125073_1    AR_T878A_region              SNP               WT         62           0          0.0000  .          .          .                0.2238
SRR26125073_1    AR_const_exon1               CONSTITUTIVE      EXPRESSED  145          .          .       .          .          .                0.5235
SRR26125073_1    GAPDH_HK153                  HOUSEKEEPING      EXPRESSED  1482         .          .       .          .          .                .
```

**Call values:**

| Type | Call | Condition |
|------|------|-----------|
| SPLICE_JUNCTION | PRESENCE | total_reads ≥ `--min-reads` |
| SPLICE_JUNCTION | ABSENCE | total_reads ≥ `--min-coverage` but < `--min-reads` |
| SPLICE_JUNCTION | INDETERMINATE | total_reads < `--min-coverage` |
| SNP | MUT | alt at expected codon, VAF ≥ `--min-mut-vaf` |
| SNP | WT | no qualifying alt, total_reads ≥ `--min-coverage` |
| SNP | INDETERMINATE | total_reads < `--min-coverage` |
| HOUSEKEEPING / CONSTITUTIVE | EXPRESSED | total_reads ≥ `--min-reads` |
| HOUSEKEEPING / CONSTITUTIVE | INDETERMINATE | total_reads < `--min-reads` |

**`splice_fraction`** = `AR_V7_reads / (AR_V7_reads + AR_FL_reads)`. Emitted only on the AR-V7 junction row; `.` elsewhere. Requires both V7 and FL targets in the reference.

**`expr_index`** = `total_reads / median(HK_reads)`. Requires `--housekeeping`; `.` if no HK reference or HK median is zero.

**`vaf_ci_lo` / `vaf_ci_hi`** — Wilson 95% confidence interval on the per-position VAF.

### `<sample>.novel.tsv`

SNP positions with VAF ≥ `--novel-min-vaf` that do not match any known expected mutation. Useful for detecting unexpected resistance variants within the target windows.

### `<sample>.tsv`

Full per-position pileup — all variant calls before summary aggregation.

### Provenance header

Every output file begins with a `#` comment line:

```
# mira=0.2.0 ref=... ref_md5=... r1=... r1_md5=size=N params=[...] timestamp=...
```

Reference MD5 is cryptographic (md5 crate). FASTQ uses file-size fingerprint (full MD5 is too slow for multi-GB inputs).

---

## Validation

Tested on 22Rv1 prostate carcinoma cell line RNA-seq (SRP400955, Illumina, paired-end 61 bp):

| Sample | AR-V7 reads | splice_fraction | H875Y | T878A / L702H / W742C / F877L | HK median |
|--------|-------------|-----------------|-------|-------------------------------|-----------|
| SRR26125073 | PRESENCE 102 (expr_index=0.37) | 0.39 | MUT 1.0 [0.91, 1.0] | WT | 277 |
| SRR26125085 | PRESENCE 142 (expr_index=0.46) | 0.50 | MUT 1.0 [0.87, 1.0] | WT | 307 |

H875Y VAF = 1.0 is consistent with a clonal resistance mutation in a highly AR-expressing cell line. PCR dedup removed ~75% of raw hits (identical sequences per target), reflecting the high duplication rate typical of RNA-seq. `splice_fraction` 0.39–0.50 indicates 39–50% of exon3-spanning reads use the AR-V7 splice site versus the canonical AR-FL exon4 junction.

**Runtime** (WSL2, uncompressed paired FASTQ ~6.5 GB each):

| Input | Time |
|-------|------|
| 12.5 GB paired FASTQ (9 AR targets + 5 HK) | ~3 min |

Peak RAM: ~35 MB regardless of input size.

---

## How it works

```
FASTQ → k-mer pre-filter → mismatch scan → PCR dedup → best-target assignment → Smith-Waterman alignment → identity gate → pileup → variant calls → HK normalization
```

1. **Index** — FNV-1a canonical k-mer hash of all target sequences (k=31 default); AR and HK targets in one shared index
2. **Pre-filter** — discard read pairs with no exact k-mer match (~99.9% of reads); ~40× speedup
3. **Scan** — mismatch-tolerant k-mer lookup (Hamming ≤ 2) with mate rescue
4. **Dedup** — sequence-level deduplication per target (`(target_id, seq)` key); removes PCR duplicates without BAM
5. **Best-target** — for reads matching multiple targets, score each SW alignment and keep only candidates within 30 score points of the best. Junction-spanning reads (score gap ~124 pts) are uniquely assigned; overlapping SNP windows (gap ≤ 18 pts) are kept in all matching targets so each variant position is evaluated independently.
6. **Align** — semiglobal Smith-Waterman (rust-bio) of each extracted read against its target
7. **Identity gate** — drop reads with alignment identity < 90% or soft-clip > 30% of read length
8. **Pileup** — per-position base counts with per-base quality filter
9. **Call** — SNP/presence calls with Wilson 95% CI; INDETERMINATE below coverage threshold
10. **Normalize** — `expr_index = target_reads / median(HK_reads)` for each non-HK target; `splice_fraction = V7_reads / (V7 + FL_reads)` on the V7 row

Reads processed in streaming 50K-pair batches — constant memory regardless of FASTQ size.

---

## Limitations

- Detects variants only within indexed target windows (~150 bp per target). Mutations elsewhere in AR are invisible without adding reference sequences.
- Novel variant output at 10% VAF includes systematic background noise; VAF ≥ 30% recommended for high-confidence clinical calls.
- CE3 sequence sourced from FJ235916.1 (Guo et al. 2009) — validate against direct amplicon sequencing before clinical use.
- SNP targets cover exonic LBD region only; intronic variants not detectable from RNA-seq.
- No ARv567es, AR-V3, or AR-V9 targets yet (straightforward to add).
- `expr_index` normalizes to constitutive HK expression; very low AR expression may produce expr_index < 0.05 regardless of variant status.
- Not validated on FFPE RNA or low-input samples.

---

## References

1. Antonarakis ES et al. *N Engl J Med.* 2014;371:1028–38 — AR-V7 and ARPI resistance
2. Romanel A et al. *Sci Transl Med.* 2015;7:312re10 — T878A/L702H at abiraterone progression
3. Joseph JD et al. *Proc Natl Acad Sci.* 2013;110:2987–92 — F877L enzalutamide resistance
4. Hwangbo et al. AACR 2023 — ≥10 junction reads threshold
5. Zurita AJ et al. *J Clin Oncol.* 2025 — FoundationOne RNA AR-V7 at ≥10 reads
6. Chen J et al. *BMC Bioinformatics.* 2018;19:16 — MutScan (k-mer scan design)
7. Guo Z et al. *Cancer Res.* 2009;69:2305–13 — AR-V7 CE3 sequence (FJ235916.1)

---

## License

MIT
