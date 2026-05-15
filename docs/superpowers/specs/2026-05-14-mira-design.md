# MIRA — Mutation In RNA-seq Aligner
## Design Spec — 2026-05-14

---

## Overview

MIRA is a Rust CLI tool for alignment-free detection of gene mutations and splice variants directly from bulk RNA-seq FASTQ files. Users provide target sequences (FASTA), MIRA scans reads via k-mer matching, extracts hits, aligns them locally, and reports variant calls in a TSV.

Designed as a generic tool — not disease- or gene-specific. First validated use case: androgen receptor (AR) mutations in prostate cancer RNA-seq (AR-V7, T878A, L702H, etc.).

---

## CLI

```
mira \
  --input/-i       <R1.fastq[.gz]> [R2.fastq[.gz]]   # 1 or 2 files
  --reference/-r   <targets.fasta>                    # target sequences
  --output/-o      <results.tsv>
  --kmer-size/-k   <int>     default: 31
  --max-mismatches/-m <int>  default: 2
  --min-reads      <int>     default: 10
  --min-mean-qual  <int>     default: 20  (read-level Phred filter)
  --min-base-qual  <int>     default: 20  (pileup-level Phred filter)
  --library-type   unstranded|forward|reverse  default: unstranded
  --save-extracted <path>    optional: write matching reads to FASTQ
  --threads/-t     <int>     default: num_cpus
```

Defaults sourced from literature: k=31 (KvarQ/MutScan), mismatches=2 (MutScan), min-reads=10 (Hwangbo et al. 2023, Zurita et al. 2025), qual=20 (GATK best practices).

---

## Architecture

Multi-threaded streaming pipeline. No intermediate files required (unless `--save-extracted` set).

```
Reader (needletail)
  └─ streams FASTQ/FASTQ.gz chunks, exposes quality bytes natively

Quality Filter
  └─ discard reads with mean Phred < --min-mean-qual
  └─ paired-end: filter both mates if either fails (keeps pairs in sync)

K-mer Scanner  [Rayon parallel worker pool]
  └─ Rabin-Karp rolling hash over read sequence (O(1) update per position)
  └─ lookup in k-mer index (HashMap<u64, Vec<TargetId>>, built at startup)
  └─ mismatch neighbors: Hamming distance ≤ --max-mismatches
      (k * 3 * d probes max; k=31, d=2 → ~186 probes per k-mer miss)
  └─ strand filter: canonical k-mers for unstranded; for forward/reverse
      stranded libraries, only hash the strand matching the expected read
      orientation (forward: R1=antisense→rc, R2=sense; reverse: opposite).
      Antisense reads excluded before k-mer lookup.
  └─ mate rescue (paired-end): R1 and R2 processed as pairs in same chunk;
      if either mate hits k-mer filter → both mates sent to aligner

Local Aligner
  └─ Smith-Waterman (bio crate) of each extracted read vs full target sequence
  └─ returns alignment position + CIGAR-equivalent pileup

Variant Caller
  └─ pileup allele counts per target position
  └─ skip bases with Phred < --min-base-qual
  └─ call variants where alt_reads >= --min-reads
  └─ annotate: SNP | SPLICE_JUNCTION | INDEL | PRESENCE

TSV Writer
  └─ one row per variant call
```

---

## K-mer Index

Built at startup from reference FASTA:

- Parse all target sequences
- Slide window of size k over each sequence
- Hash each k-mer canonically: `min(hash(fwd), hash(rev_comp))`
- Store: `HashMap<u64, Vec<TargetId>>`
- One k-mer can map to multiple targets (handles overlapping sequences)

---

## Output TSV Schema

```
sample  target_id  variant_type  position  ref  alt  supporting_reads  total_reads  frequency
```

- `sample`: filename stem of input FASTQ
- `target_id`: sequence ID from reference FASTA
- `variant_type`: `SNP` | `SPLICE_JUNCTION` | `INDEL` | `PRESENCE`
  - `SPLICE_JUNCTION`: target sequence in FASTA is a junction-spanning sequence (e.g. exon3–cryptic-exon junction); reads aligning = junction detected. No pileup variant needed — PRESENCE of alignment is the signal.
  - `SNP` / `INDEL`: detected via pileup mismatch/gap at target position.
  - `PRESENCE`: target detected but no specific variant called (supporting_reads = aligned read count).
- `position`: 1-based position in target sequence
- `ref` / `alt`: reference and alternate allele (`.` for PRESENCE calls)
- `supporting_reads`: reads supporting alt allele
- `total_reads`: total reads mapped to target
- `frequency`: supporting_reads / total_reads

If no variants found for a target: one row with variant_type=`PRESENCE`, supporting_reads=0.

---

## Module Structure

```
src/
  main.rs       CLI entry point, wires pipeline stages
  cli.rs        clap arg definitions and validation
  index.rs      K-mer index build from reference FASTA
  scanner.rs    Rayon FASTQ scan: quality filter, rolling hash, strand filter, mate rescue
  aligner.rs    Smith-Waterman alignment + pileup construction
  caller.rs     Variant calling from pileup, base quality filter, annotation
  output.rs     TSV writer
  types.rs      Shared structs: Read, Hit, Pileup, Variant, TargetId
```

---

## Dependencies

| Crate       | Purpose                              |
|-------------|--------------------------------------|
| `clap`      | CLI argument parsing                 |
| `needletail` | FASTQ/FASTQ.gz streaming parser     |
| `rayon`     | Parallel worker pool                 |
| `bio`       | Smith-Waterman alignment             |
| `ahash`     | Fast non-cryptographic HashMap       |
| `flate2`    | Gzip decompression (via needletail)  |
| `anyhow`    | Error propagation with context       |

---

## Error Handling

- Invalid FASTQ → exit with message + line number
- Reference FASTA parse failure → exit immediately
- Zero k-mer hits → warn to stderr, write empty TSV (no silent failure)
- Paired-end read count mismatch → error + abort
- All errors via `anyhow`, propagated with `?`, printed with full context chain

---

## Testing

- Unit tests per module with synthetic reads containing known mutations
- Integration test: synthetic FASTQ + reference FASTA → assert TSV matches expected variants
- Edge cases: empty FASTQ, all reads quality-filtered, no k-mer hits, mismatched pair counts
- Benchmark: track throughput (reads/sec) on representative dataset

---

## Literature Basis

| Design Choice              | Source                          |
|----------------------------|---------------------------------|
| K-mer size 31              | KvarQ (Steiner 2014), MutScan (Chen 2018) |
| Max mismatches 2           | MutScan (Chen 2018)             |
| Min reads 10               | Hwangbo 2023, Zurita 2025       |
| Rolling hash               | MutScan (Chen 2018)             |
| Strand specificity         | ARscan summary — technical challenge #3 |
| Mate rescue                | MutScan (Chen 2018)             |
| Base quality filter Q20    | GATK best practices             |
| Smith-Waterman local align | Standard; MutScan, KvarQ precedent |
