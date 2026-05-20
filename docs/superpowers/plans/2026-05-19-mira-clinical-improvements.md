# MIRA Clinical Readiness Plan

Biological + technical improvement plan for `/MIRA/` aimed at turning MIRA from a research prototype into a clinic-ready tool for AR-directed therapy selection in mCRPC.

**Goal**: FASTQ in → AR mutations detected → therapy recommendation out (Taxane / Darolutamide / Continue ARPI / Uncertain)

---

## 1. Current capabilities assessment

### Targets detected
Six hard-coded sequences in `AR_targets.fa`:
- **Splice**: `AR_V7_exon3_CE3_junction`, `AR_CE3_full`
- **SNP**: `AR_T878A`, `AR_L702H`, `AR_W742C`, `AR_H875Y`

Targets are encoded as ~150–160 bp synthetic windows; the expected codon change is embedded in the FASTA header (`offset=…_ref=ACT_alt=GCT`) and parsed at output time by `parse_target_meta` in `src/output.rs:59-79`.

### Input formats
- Single- or paired-end FASTQ (gzipped via `flate2` + `needletail`).
- No SAM/BAM/CRAM input. No UMI awareness. No FFPE-specific handling.
- `--library-type {unstranded | forward | reverse}` (`src/cli.rs:4-9`, `src/scanner.rs:204-216`).

### Algorithm
A k-mer pre-filter + Smith-Waterman pileup pipeline (no genome alignment, no pseudoalignment):

1. **Indexing** (`src/index.rs:14-41`): canonical FNV-1a hash over every k-mer in the reference FASTA (default k=31).
2. **Pre-filter** (`src/scanner.rs:74,162`): drop reads with zero exact k-mer hit against the target hash.
3. **Mismatch scan** (`src/index.rs:59-135`): probe each k-mer position for ≤2 Hamming-distance neighbors.
4. **Align** (`src/aligner.rs:75-79`): banded semi-global SW from `rust-bio` (match +2, mismatch −2, gap open −4, gap extend −1).
5. **Pileup** (`src/aligner.rs:21-72`): per-position counts, filtered by per-base Phred ≥ 20.
6. **Call** (`src/caller.rs`): any position with ≥ `--min-reads` alt support → SNP call. Splice targets get `PRESENCE` when ≥ `--min-reads` align.

### Outputs
Three TSVs: `<out>.tsv` (raw pileup), `<out>.summary.tsv` (one row per target), `<out>.novel.tsv` (unexpected high-VAF SNPs).

### Tests
Three end-to-end smoke tests in `tests/integration_test.rs`. No fidelity tests, no FPR characterization, no AR-V7 junction-specific tests, no truth-set comparisons.

---

## 2. Biological gaps for clinical use

### 2.1 AR splice variants beyond AR-V7

**Missing**: Only AR-V7 (CE3 junction + CE3 body) is targeted.

**Clinical impact**: The broader splice landscape matters:
- **AR-V9 (CE5)**: co-expressed with AR-V7, independently associated with abiraterone resistance (Kohli 2017 EBioMedicine). In some patients AR-V9 is dominant and AR-V7 negative.
- **ARv567es (exon-skip 5/6/7)**: constitutively active, abundant in CRPC bone metastases (Sun 2010, Hörnberg 2011). Distinct mechanism (intra-exon splice, not cryptic exon).
- **AR-V3, AR-V1**: minor variants, but AR-V3 has constitutive activity.

**Implementation**:
- Add targets for AR-V9 (exon3→CE5 junction), ARv567es (exon4→exon8 junction), AR-V3 (exon2b junction) to `AR_targets.fa`.
- Report derived **AR-V class call**: `AR-FL only | AR-V positive | AR-V dominant`.

### 2.2 AR-V7 quantification (ratio, not binary)

**Missing**: AR-V7 is binary (`PRESENCE` if reads ≥ 10). No AR-V7/AR-FL ratio.

**Clinical impact**: Background CE3 transcription occurs in many CRPC samples. The clinically relevant signal is **relative AR-V7 abundance**. ≥10 junction reads is a sensitivity threshold, not a specificity one.

**Implementation**:
- Add `AR_FL_exon3_exon4_junction` reference target (canonical exon 3 → exon 4 splice).
- Report `AR-V7_reads / (AR-V7_reads + AR-FL_reads)` as AR-V7 splicing index.
- Include 95% Wilson CI. Thresholds from literature: ≥0.10 high concern; ≥0.05 intermediate.

### 2.3 Missing point mutations

**Missing from current panel**:

| Mutation | Mechanism | Clinical relevance |
|----------|-----------|---------------------|
| **F877L** | Anti-androgen converter | Enzalutamide/apalutamide resistance — **OPPOSITE directionality from L702H** (Joseph 2013, Korpal 2013) |
| **T878S** | LBD promiscuity | Different codon than T878A; distinct ARPI sensitivity |
| **H875Y + T878A compound** | Compound | Strong abiraterone resistance (Romanel 2015 STM) |
| **E709K, V716M, M896V** | Rare LBD | Emerging in cfDNA studies |

**Implementation**: Add `F877L (TTC→CTC)`, `T878S (ACT→AGT)`, `E709K`, `M896V` to `AR_targets.fa`. Report compound H875Y+T878A by checking phased haplotypes on individual reads.

### 2.4 AR amplification / overexpression

**Missing**: No AR copy-number or expression readout.

**Clinical impact**: AR amplification (~50% of mCRPC) is the most common ARPI resistance driver after castration.

**Implementation**:
- Add housekeeping reference targets (GAPDH, ACTB, HPRT1, TBP, B2M).
- Report `AR_FL / median(housekeeping)` as relative expression z-score vs reference cohort.
- Document clearly: this is *expression*, not copy number.

### 2.5 Confidence and statistical uncertainty

**Missing**: No CIs on VAF, no per-call posterior, no strand bias, no read-position bias. `--min-mut-vaf 0.30` is a hard gate.

**Clinical impact**: `MUT` at 31% VAF / 12 reads ≠ `MUT` at 50% VAF / 400 reads — currently identical in output.

**Implementation**:
- **Wilson 95% CI** on every VAF in `summary.tsv`.
- Per-variant **strand bias** (Fisher exact on fwd/rev alt counts).
- **Read-position bias** (mean position of alt base within read).
- Three-tier call: `MUT (high) | MUT (low-confidence) | WT | INDETERMINATE`.

### 2.6 Background / noise floor calibration

**Missing**: README acknowledges 10–16% systematic background VAF but does not subtract or model it.

**Implementation**:
- Ship empirical noise profile from a reference WT panel (TCGA normals or 22Rv1 WT positions).
- Store per-target, per-position background β-distribution parameters in `noise_profile.tsv`.
- At call time, use Beta-Binomial LRT vs background. Removes the heuristic 30% threshold.

### 2.7 Reference allele and transcriptome version

**Missing**: No version metadata. T878 vs T877 numbering ambiguity between annotation versions.

**Clinical impact**: A clinical report must state "AR NM_000044.6:p.Thr878Ala (chrX:g.67723701A>G)" with HGVS-p, HGVS-c, HGVS-g and GRCh38 coordinates for ClinVar/OncoKB interoperability.

**Implementation**: Embed `transcript=NM_000044.6 hgvsc=c.2632A>G hgvsp=p.Thr878Ala genome=GRCh38 chr=X pos=67723701` in FASTA headers; output HGVS notation and VCF.

### 2.8 Three-state call: INDETERMINATE

**Missing**: Below `--min-reads`, target silently becomes `ABSENCE` or `WT`. Cannot distinguish "tested and negative" from "not enough data."

**Clinical impact**: A `WT` call at 3 reads is not WT — it is **non-evaluable**. False reassurance has direct clinical consequences.

**Implementation**:
- Three-state call: `MUT / WT / INDETERMINATE-LOW-COVERAGE`.
- Coverage gate: warn `INDETERMINATE` when `total_reads < 30` (at 10 reads, sensitivity to 30% VAF is only ~72%).
- Emit sample-level `evaluable_targets / total_targets`.

### 2.9 RNA-specific artifacts

**Missing**: No FFPE deamination model, no PCR-duplicate marking, no ADAR edit awareness.

**Clinical impact**:
- **FFPE**: systematic C>T / G>A artifacts from cytosine deamination. **AR L702H (CTC→CAC) sits on the C>T strand** — classic FFPE false positive.
- **T878A is A>G in cDNA** — overlaps with ADAR editing pattern. ADAR editing of AR has been reported in CRPC.
- PCR duplicates inflate alt counts without UMI deduplication.

**Implementation**:
- `--ffpe` mode: down-weights C>T and G>A calls < 30% VAF.
- Read deduplication by sequence + position before pileup.
- Flag T878A calls < 25% VAF with `POSSIBLE_RNA_EDIT` unless DNA confirmation performed.

### 2.10 Tumor purity

**Missing**: No estimate of tumor-derived signal fraction.

**Clinical impact**: Bone biopsies have 20–60% tumor cellularity. A WT call at 5% tumor fraction is meaningless (50% clonal mutation diluted to 2.5% VAF).

**Implementation**:
- CLI flag `--tumor-purity 0.4` propagates as VAF prior in LRT.
- Emit `effective_LoD_VAF = noise_floor / tumor_purity` in report header.

---

## 3. Technical / algorithmic improvements

### 3.1 Critical bugs in k-mer + SW path

- **`src/scanner.rs`**: a read can be assigned to multiple targets and pileup into multiple targets simultaneously — double-counts reads when targets overlap. **Fix**: choose best target per read by max alignment score.
- **`src/aligner.rs:75-79`**: no alignment identity gate — any aligned read contributes to pileup regardless of quality, soft-clip fraction, or score. **Fix**: drop reads with identity < 90%, alignment score < 0.6×max, or soft-clip > 30%.
- **Mate rescue too aggressive** (`src/scanner.rs:170-184`): R2 can pileup into a target it has no overlap with. **Fix**: per-read alignment-gating downstream of mate extraction.

### 3.2 Reference metadata validation (`src/output.rs:48-78`)

The FASTA-header parsing of `ref=ACT alt=GCT` is fragile — no validation that embedded codon matches actual FASTA sequence at the stated offset. A typo silently produces wrong calls.

**Fix**: at index build time, validate codon against FASTA sequence; fail loudly on mismatch. Move metadata to a sidecar `targets.yaml`.

### 3.3 Deterministic output ordering

The parallel `Mutex<Vec<Hit>>` accumulation does not guarantee deterministic output ordering. Required for clinical reproducibility and CLIA compliance.

**Fix**: sort final output by target ID + position before writing.

### 3.4 Target registry refactor

Replace FASTA-header-encoded metadata with a typed registry (`targets.yaml`):

```yaml
- id: AR_T878A_region
  category: LbdSnp
  transcript: NM_000044.6
  hgvs_c: c.2632A>G
  hgvs_p: p.Thr878Ala
  genome: GRCh38
  chr: X
  pos: 67723701
  ref_codon: ACT
  alt_codon: GCT
  therapy_link: Darolutamide
```

### 3.5 FFPE / degraded RNA

- `--ffpe` flag: relax mean-quality threshold, enable short-read mode (min 36 bp), apply C>T/G>A filter.
- `--min-read-length` flag — currently anything ≥ k is accepted without floor.

---

## 4. Clinical validation requirements

### 4.1 Truth set needed

- **AR-V7**: concordance with AdnaTest AR-V7, Epic Sciences, or FoundationOne RNA. n ≥ 100. PPA/NPA each ≥ 95% (lower CI bound ≥ 85%).
- **LBD mutations**: concordance with FoundationOne CDx, Guardant360, or Tempus xT on matched tumor/cfDNA.
- **LoD**: titration of AR-V7+ (22Rv1) into AR-V7− (LNCaP) RNA at 1%, 5%, 10%, 25%, 50%.
- **Repeatability**: 3 operators × 3 days × 3 instruments.
- **Interference**: hemolysis, low RIN, FFPE blocks of varying age.

### 4.2 Regulatory pathway

- **LDT under CLIA/CAP** (most likely starting point): full analytical validation, CAP molecular checklist, proficiency testing.
- **FDA** (if marketed): 510(k) or PMA. Companion diagnostic claims require co-development with drug sponsor.
- **EU IVDR Class C**: notified body review, performance evaluation report, post-market surveillance plan.
- **Software**: ISO 13485 quality system, IEC 62304 Class B software lifecycle. Output must be deterministic.

---

## 5. Output / reporting improvements

### 5.1 Clinical report should contain

1. Header: sample ID, MIRA version, reference version, GRCh38 build, parameters, run datetime, FASTQ md5.
2. Per-target: HGVS-p, HGVS-c, HGVS-g, VAF + 95% Wilson CI, supporting/total reads, call (MUT/WT/INDETERMINATE), confidence, background FPR.
3. AR-V landscape: AR-V7 ratio, AR-V9 ratio, ARv567es ratio, AR-FL expression z-score.
4. Therapy interpretation: structured logic tree with literature citations.
5. Caveats: tumor purity, low-coverage targets, RNA-editing flags, FFPE warnings.

### 5.2 Structured outputs to add

- **VCF v4.3** with proper `CHROM/POS` (GRCh38), `INFO/VAF`, `INFO/AD`, `INFO/DP`, standard `FILTER` tags.
- **JSON** machine-readable summary (primary output for dashboard integration).
- Preserve current **TSV** for backward compatibility.

### 5.3 Wilson CI formula

```
vaf_lo = (p + z²/(2n) − z√(p(1−p)/n + z²/(4n²))) / (1 + z²/n)
vaf_hi = (p + z²/(2n) + z√(p(1−p)/n + z²/(4n²))) / (1 + z²/n)
```
where z = 1.96 for 95% CI.

---

## 6. Prioritized roadmap

### P0 — Required before any clinical use

| Item | Effort | Location |
|------|--------|----------|
| Three-state call (MUT/WT/INDETERMINATE) with explicit low-coverage handling | S (1d) | `src/output.rs:141-209`, `src/caller.rs` |
| Wilson 95% CI on every VAF in summary | S (1d) | `src/output.rs` |
| Validate reference codons against FASTA sequence at startup | S (1d) | `src/index.rs`, `src/output.rs:48-78` |
| Add F877L target (`TTC→CTC`) — enzalutamide resistance | S (4h) | `AR_targets.fa` |
| Add AR-FL (exon3→exon4) target + AR-V7/AR-FL ratio | M (2d) | `AR_targets.fa`, `src/output.rs` |
| Alignment identity gate (drop reads < 90% identity, > 30% soft-clip) | S (1d) | `src/aligner.rs:75-79` |
| Best-target assignment per read (fix multi-target pileup) | M (2d) | `src/scanner.rs`, `src/aligner.rs` |
| Deterministic output ordering (sort by target id + position) | S (2h) | `src/output.rs`, `src/caller.rs` |
| Provenance header in all outputs (MIRA version, reference md5, parameters, FASTQ md5, timestamp) | S (1d) | `src/output.rs`, `src/main.rs` |
| Tests: AR-V7 true-positive, FPR (1000 random reads → 0 false calls), low-coverage → INDETERMINATE | M (2d) | `tests/integration_test.rs` |

### P1 — High value for clinical credibility

| Item | Effort |
|------|--------|
| Add AR-V9, ARv567es splice targets | M (2d biology + 1d code) |
| Add T878S, compound H875Y+T878A, E709K targets | M (2d) |
| Empirical noise-profile model (per-target β-binomial background) — needs WT reference panel | L (1–2w) |
| Strand bias + read-position bias filters | M (3d) |
| VCF v4.3 output with HGVS notation and GRCh38 coordinates | M (3d) |
| JSON structured output for dashboard integration | S (1d) |
| FFPE mode (`--ffpe` flag, C>T/G>A down-weighting) | M (2d) |
| BAM/CRAM input via `rust-htslib` | M (3d) |
| Read deduplication (sequence + position) before pileup | S (1d) |
| Phased haplotype reporting for compound LBD mutations | M (3d) |
| Target registry refactor (targets.yaml, drop FASTA-header parsing) | M (2–3d) |
| Replace `Mutex<Vec<Hit>>` with thread-local rayon accumulators | S (1d) |
| Validation study: 22Rv1 dilution series LoD; concordance vs FoundationOne RNA on ≥ 20 samples | L (months — wet-lab) |

### P2 — Nice to have / longer term

| Item | Effort |
|------|--------|
| AR amplification/overexpression module (housekeeping reference panel) | M |
| `mira aggregate` longitudinal multi-sample subcommand | M |
| UMI-aware deduplication | M |
| ADAR-edit detection / annotation | M |
| FHIR Genomics Reporting export | L |
| ISO 13485 / IEC 62304 documentation package for regulatory submission | XL |
| Full JSON contract with mira-dashboard covering CIs, INDETERMINATE, AR-V ratios | M (2d) |
| PDF clinical report with signing block | M |

---

## Key files

| File | Issue |
|------|-------|
| `src/index.rs:14-41` | k-mer indexing — no target-overlap validation |
| `src/scanner.rs:74,162` | pre-filter — a read can match multiple targets |
| `src/scanner.rs:170-184` | mate rescue too aggressive |
| `src/aligner.rs:75-79` | no alignment identity gate |
| `src/aligner.rs:21-72` | pileup — no strand tracking |
| `src/caller.rs` | binary call only, no INDETERMINATE |
| `src/output.rs:48-78` | fragile FASTA-header metadata parsing |
| `src/output.rs:141-209` | summary thresholding — no CI, no 3-state |
| `src/cli.rs:4-9` | missing `--ffpe`, `--tumor-purity`, `--min-identity`, `--target-manifest` flags |
| `tests/integration_test.rs` | only 3 smoke tests — no FPR, no AR-V7 junction, no low-coverage |
| `AR_targets.fa` | missing F877L, AR-V9, ARv567es, AR-FL, T878S, compound targets |

---

## Bottom line

MIRA is a clean, fast research prototype. The biggest gaps are **interpretive and statistical**, not algorithmic:

1. No `INDETERMINATE` state — silently calls low-coverage as WT
2. No confidence intervals — 30% VAF threshold is a research heuristic
3. Missing F877L — enzalutamide resistance (therapy-altering omission)
4. No HGVS/VCF output — blocks interoperability with ClinVar/OncoKB
5. FASTA-header metadata has no validation — typo silently breaks calls

Fix P0 → defensible LDT candidate. Add P1 → competitive with commercial AR-V7/AR-LBD assays at a fraction of the cost.
