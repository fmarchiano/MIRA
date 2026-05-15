# MIRA

**Mutation In RNA-seq Aligner** — alignment-free androgen receptor variant detection from bulk RNA-seq FASTQ.

MIRA scans FASTQ files directly for AR splice variants (AR-V7) and resistance mutations (T878A, L702H, W742C, H875Y) without reference genome alignment. It extracts AR-matching reads via k-mer indexing, aligns them with Smith-Waterman, and calls variants from pileup — producing three output tiers per sample.

---

## Clinical motivation

AR status determines treatment in metastatic castration-resistant prostate cancer (mCRPC):

| Finding | Implication |
|---------|-------------|
| AR-V7+ | Switch to taxane (enzalutamide/abiraterone: 0% PSA response) |
| T878A / L702H | Abiraterone resistance — switch to darolutamide |
| AR-V7− / no LBD mutations | Continue ARPI |

MIRA runs on existing bulk RNA-seq data — no dedicated assay required.

---

## Installation

### Pre-compiled binary (Linux x86_64, glibc ≥ 2.34)

```bash
curl -LO https://github.com/fmarchiano/MIRA/releases/latest/download/mira
curl -LO https://github.com/fmarchiano/MIRA/releases/latest/download/AR_targets.fa
chmod +x mira
./mira --help
```

### Build from source

**Requirements:** Rust ≥ 1.75, Cargo

```bash
git clone https://github.com/fmarchiano/MIRA.git
cd MIRA
cargo build --release
# binary: ./target/release/mira
```

---

## Quick start

```bash
# Paired-end RNA-seq
mira \
  -i sample_R1.fastq sample_R2.fastq \
  -r reference/AR_targets.fa \
  -o results/sample_AR.tsv \
  --library-type unstranded \
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
| `sample_AR.summary.tsv` | One row per target: PRESENCE/ABSENCE or WT/MUT |
| `sample_AR.novel.tsv` | High-VAF variants not matching any expected mutation |

---

## Reference FASTA

Download `AR_targets.fa` from the [latest release](https://github.com/fmarchiano/MIRA/releases/latest):

```bash
curl -LO https://github.com/fmarchiano/MIRA/releases/latest/download/AR_targets.fa
```

Then pass it via `-r AR_targets.fa`. It contains six synthetic 150–160 bp sequences centered on clinically actionable AR sites:

| Target | Type | Source |
|--------|------|--------|
| AR_V7_exon3_CE3_junction | Splice junction | NM_000044.6 + FJ235916.1 |
| AR_CE3_full | CE3 cryptic exon | FJ235916.1 |
| AR_T878A_region | SNP (ACT→GCT) | NM_000044.6 |
| AR_L702H_region | SNP (CTC→CAC) | NM_000044.6 |
| AR_W742C_region | SNP (TGG→TGT) | NM_000044.6 |
| AR_H875Y_region | SNP (CAT→TAT) | NM_000044.6 |

Additional targets (ARv567es, AR-V3, AR-V9, novel mutations) can be added by appending sequences to the FASTA. Each target should be ~150 bp centered on the variant site (75 bp flanking per side, k=31).

---

## Options

```
Usage: mira [OPTIONS] --input <INPUT>... --reference <REFERENCE> --output <OUTPUT>

Options:
  -i, --input <INPUT>...          Input FASTQ file(s): one (single-end) or two (R1 R2, paired-end)
  -r, --reference <REFERENCE>     Reference target sequences (FASTA)
  -o, --output <OUTPUT>           Output TSV file (summary and novel TSVs auto-derived)
  -k, --kmer-size <K>             K-mer size [default: 31]
  -m, --max-mismatches <M>        Max Hamming mismatches for k-mer lookup [default: 2]
      --min-reads <N>             Min supporting reads to call a variant [default: 10]
      --min-mean-qual <Q>         Min mean Phred quality to keep a read [default: 20]
      --min-base-qual <Q>         Min per-base Phred quality in pileup [default: 20]
      --library-type <TYPE>       Library strandedness: unstranded | forward | reverse [default: unstranded]
      --min-mut-vaf <VAF>         Min VAF to call a known SNP as MUT in summary [default: 0.30]
      --novel-min-vaf <VAF>       Min VAF to report in novel variants output [default: 0.10]
      --save-extracted <PATH>     Write extracted AR reads to FASTQ (optional)
  -t, --threads <T>               Threads [default: logical CPUs]
  -h, --help                      Print help
```

---

## Output format

### `<sample>.summary.tsv`

One row per target. Primary output for clinical interpretation.

```
sample          target                       type              call      total_reads  alt_reads  vaf
SRR26125085_1   AR_V7_exon3_CE3_junction    SPLICE_JUNCTION   PRESENCE  1122         .          .
SRR26125085_1   AR_T878A_region             SNP               WT        202          0          0.0000
```

- `call` for splice targets: `PRESENCE` (≥ `--min-reads`) or `ABSENCE`
- `call` for SNP targets: `MUT` (alt at expected codon, VAF ≥ `--min-mut-vaf`) or `WT`

### `<sample>.novel.tsv`

SNP positions with VAF ≥ `--novel-min-vaf` that do not match a known expected mutation. Useful for detecting unexpected resistance variants within the target windows.

> **Noise floor:** systematic off-target background produces ~10–16% VAF uniformly across positions. Variants at VAF ≥ 30% at a single codon position are high-confidence calls; variants in the 10–20% range require manual review or matched-normal comparison.

### `<sample>.tsv`

Full per-position pileup — all variant calls before summary aggregation.

---

## Validation

Tested on 22Rv1 prostate carcinoma cell line RNA-seq (SRP400955, Illumina NextSeq 2000, paired-end ~100 bp):

| Target | siNON (AR-V7+) | siARV7 (knockdown) |
|--------|---------------|--------------------|
| AR_V7_exon3_CE3_junction | **1122 reads** — PRESENCE | 870 reads — PRESENCE |
| AR_CE3_full | **716 reads** — PRESENCE | 314 reads — PRESENCE |
| AR_T878A / L702H / W742C / H875Y | WT (202–210 reads coverage) | WT (298–324 reads coverage) |

AR-V7 signal reduced 22–56% under siARV7, consistent with partial knockdown. All SNP targets correctly called WT (22Rv1 carries no documented AR point mutations).

**Runtime** (i7-8550U, 4 cores, uncompressed FASTQ):

| Input | Time |
|-------|------|
| 12.6 GB paired FASTQ | ~2 min |
| 11.8 GB paired FASTQ | ~7 min |

Peak RAM: ~35 MB regardless of input size.

---

## How it works

```
FASTQ → k-mer pre-filter → mismatch scan → Smith-Waterman alignment → pileup → variant calls
```

1. **Index** — FNV-1a canonical k-mer hash of all target sequences (k=31 default)
2. **Pre-filter** — discard read pairs with no exact k-mer match to any target (~99.9% of reads in a bulk RNA-seq); ~40× speedup vs probing all reads for mismatches
3. **Scan** — mismatch-tolerant k-mer lookup (Hamming ≤ 2) on surviving reads; mate rescue: if either read hits, both are extracted
4. **Align** — banded semiglobal Smith-Waterman (rust-bio) of each extracted read against its target sequence
5. **Pileup** — per-position base counts with per-base quality filter
6. **Call** — SNP/indel/presence calls; three-tier output

Processes reads in streaming 50K-pair batches — constant memory regardless of FASTQ size.

---

## Limitations

- Detects variants only within indexed target windows (150–160 bp per target). Mutations elsewhere in AR are invisible without adding reference sequences.
- Novel variant output at 10% VAF includes systematic background noise; VAF ≥ 30% recommended for high-confidence clinical calls.
- CE3 sequence sourced from FJ235916.1 (Guo et al. 2009) — not independently re-sequenced. Validate against direct amplicon sequencing before clinical use.
- SNP targets cover exonic LBD region only; intronic variants not detectable from RNA-seq.
- No ARv567es, AR-V3, or AR-V9 targets yet.

---

## References

1. Antonarakis ES et al. *N Engl J Med.* 2014;371:1028–38 — AR-V7 and ARPI resistance
2. Romanel A et al. *Sci Transl Med.* 2015;7:312re10 — T878A/L702H at abiraterone progression
3. Hwangbo et al. AACR 2023 — ≥10 junction reads threshold
4. Zurita AJ et al. *J Clin Oncol.* 2025 — FoundationOne RNA AR-V7 at ≥10 reads
5. Chen J et al. *BMC Bioinformatics.* 2018;19:16 — MutScan (k-mer scan design)
6. Steiner G et al. *BMC Genomics.* 2014 — KvarQ (alignment-free FASTQ variant detection)
7. Guo Z et al. *Cancer Res.* 2009;69:2305–13 — AR-V7 CE3 sequence (FJ235916.1)

---

## License

MIT
