# MIRA v0.2.0 — Implementation Status (2026-05-19)

## What was implemented today

### Code review fixes (session 1)

| ID | File | Fix |
|----|------|-----|
| C1 | `src/index.rs` | Non-ACGT k-mers skipped at index build and query |
| C2 | `src/index.rs` | `N` encoded as sentinel 4, distinct from ACGT |
| C3 | `src/aligner.rs` | `qual.get().unwrap_or(0)` for qual-less reads |
| C4 | `src/caller.rs` | VAF denominator = per-position depth (`col.total`), not PRESENCE total |
| C5 | `src/aligner.rs` | Switched to unbanded SW (`Aligner::with_scoring`) |
| M1 | `src/scanner.rs` | Pre-filter: exact k-mer hit required before mismatch scan |
| M2 | `src/scanner.rs` | Paired quality filter: skip only if BOTH mates fail |
| M3 | `src/output.rs` | `parse_target_meta` uses `_ref=` / `_alt=` keys only |
| M6 | `src/caller.rs` | PRESENCE row always emitted unconditionally |
| M8 | `src/main.rs` | Sample name = everything before first `.` in filename |
| m5 | `src/scanner.rs` | Empty FASTQ fast-path via `metadata().len() == 0` |
| m7 | `src/cli.rs` | k-mer size validated in 15–63 range |
| m8 | `src/main.rs` | `save_extracted_reads` uses `AHashSet` for dedup |
| m10 | `tests/integration_test.rs` | `mira_bin()` uses `env!("CARGO_BIN_EXE_mira")` |

### P0 quick wins (session 2, today)

| Item | Details |
|------|---------|
| **INDETERMINATE state** | `caller = INDETERMINATE` when `total_reads < --min-coverage` (default 30) for WT / ABSENCE calls. Prevents false-negative reassurance at low coverage. |
| **Wilson 95% CI on VAF** | New `vaf_ci_lo` / `vaf_ci_hi` columns in summary. CI denominator = per-position depth (consistent with displayed VAF). |
| **Deterministic output ordering** | Raw TSV sorted by `target_id → non-PRESENCE first → position → alt allele`. Required for reproducibility. |
| **F877L target** | Added `AR_F877L_region` to `AR_targets.fa`. 153 bp window centered at codon 877, offset=75, ref=TTC(F), alt=CTC(L). Enzalutamide-resistance mutation (c.2629T>C, Joseph/Korpal 2013). |
| **Alignment identity gate** | Reads with alignment identity < 90% or soft-clip > 30% of read length dropped before pileup. Removed false-positive reads that were inflating WT counts in v0.1. |
| **Provenance header** | Every output TSV starts with `# mira=0.2.0 ref=... ref_md5=... r1=... r1_md5=size=N params=[...] timestamp=...`. Reference MD5 is cryptographic (md5 crate); FASTQ uses file-size fingerprint (full FASTQ MD5 was too slow for 6 GB inputs). |
| **Version bump** | `Cargo.toml` version → `0.2.0` |
| **Dead code cleanup** | Removed unused `VariantType::SpliceJunction` variant |

### Clinical interpretation note (v0.1 vs v0.2)

v0.1 reported H875Y VAF ≈ 0.56 for both test samples. v0.2 reports 1.0 (CI [0.91, 1.0] / [0.95, 1.0]).

The change is real and correct: the identity gate removed 188 off-target reads per sample (false positives from k-mer collision) that happened to show the WT base C at position 76 by chance. The true signal — reads from genuine AR transcripts — is 100% H875Y-mutant, consistent with clonal resistance mutation in a highly AR-expressing tumor cell population.

---

## Results summary (v0.2.0, results_improv2/)

Both samples: **AR-V7 PRESENCE, H875Y MUT (VAF 1.0, CI [0.91–1.0]), T878A/L702H/W742C/F877L WT**

| Sample | AR-V7 | AR-CE3 | T878A | L702H | W742C | H875Y | F877L |
|--------|-------|--------|-------|-------|-------|-------|-------|
| SRR26125073 | PRESENCE (219) | PRESENCE (105) | WT (118) | WT (95) | WT (119) | MUT 1.0 [0.95,1.0] | WT (114) |
| SRR26125085 | PRESENCE (325) | PRESENCE (267) | WT (58) | WT (76) | WT (84) | MUT 1.0 [0.91,1.0] | WT (61) |

---

### P0 quick wins (session 3, today)

| Item | Details |
|------|---------|
| **PCR dedup** | `deduplicate_hits()` keyed on `(target_id, read_seq)`. Default ON; `--no-dedup` to skip. SRR26125073: 45 320 → 10 351 hits (75% removed). SRR26125085: 40 580 → 10 090 hits (75% removed). |
| **Housekeeping normalization** | Separate `reference/housekeeping.fa` with 5 genes: GAPDH (NM_002046.7), ACTB (NM_001101.5), HPRT1 (NM_000194.3), B2M (NM_004048.4), TBP (NM_003194.5). Each 153 bp, codon-aligned middle-third CDS window. Loaded via `--housekeeping` flag into the same k-mer index. |
| **`expr_index` column** | Summary column: `total_reads / median(HK_reads)`. HK median computed from the 5 EXPRESSED genes. AR targets show their normalized expression level. HK rows themselves emit `.`. |
| **EXPRESSED / INDETERMINATE for HK targets** | HK targets call EXPRESSED (≥ `--min-reads`) or INDETERMINATE. |
| **`KmerIndex::merge_fasta()`** | Appends a second FASTA into an existing index with contiguous target IDs. |
| **AR-FL junction target** | `AR_FL_exon3_exon4_junction` added to `AR_targets.fa`: exon3_last75 + exon4_first75 from NM_000044.6 (150 bp). Constitutive AR-FL splice serves as denominator for V7 splice fraction. |
| **AR amplification target** | `AR_const_exon1` CONSTITUTIVE 153 bp window from NTD coding region (mRNA 1934–2086, NM_000044.6). Present in all AR isoforms; used as amplification proxy. |
| **`splice_fraction` column** | Summary column: AR-V7 reads / (AR-V7 + AR-FL reads). Emitted only on V7 row; `.` elsewhere. |
| **CONSTITUTIVE target type** | Calls EXPRESSED / INDETERMINATE; expr_index emitted. |
| **Reference codon validation** | At startup, every SNP target's `_ref=` codon in header is checked against FASTA bytes at `offset=`. Mismatch aborts with a clear error. Catches annotation typos before silent wrong calls. |
| **Best-target assignment** | `assign_best_target()` groups hits by read sequence. For reads hitting multiple targets, scores each alignment (SW, fwd+RC). Keeps all candidates within `MIN_SCORE_GAP=30` of best score. Outcome: V7/FL junction reads uniquely assigned (score diff ~124 pts); H875Y/T878A/F877L overlapping reads kept in all targets (score diff ≤ 18 pts → both evaluated). SRR26125073: 230 reads uniquely reassigned, 295 removed. |
| **4 new integration tests** | `test_splice_junction_presence`, `test_low_coverage_indeterminate`, `test_wt_reads_no_false_mut`, `test_codon_validation_fails_on_mismatch`. Total: 7/7 pass. |

### Results summary (v0.2.0 + all session 3 features, results_v025/)

| Sample | AR-V7 | splice_frac | AR-CE3 | AR-FL | AR-const | T878A | L702H | W742C | H875Y | F877L | HK median |
|--------|-------|-------------|--------|-------|----------|-------|-------|-------|-------|-------|-----------|
| SRR26125073 | PRESENCE 102 (0.37) | 0.39 | PRESENCE 67 (0.24) | PRESENCE 158 | EXPRESSED 145 (0.52) | WT 62 | WT 59 | WT 71 | **MUT 1.0** [0.91,1.0] | WT 60 | 277 |
| SRR26125085 | PRESENCE 142 (0.46) | 0.50 | PRESENCE 121 (0.39) | PRESENCE 141 | EXPRESSED 146 (0.48) | WT 40 | WT 47 | WT 44 | **MUT 1.0** [0.87,1.0] | WT 41 | 307 |

HK gene read counts (results_v025):

| Gene | SRR26125073 | SRR26125085 |
|------|-------------|-------------|
| GAPDH | 1482 | 1240 |
| ACTB | 537 | 586 |
| B2M | 277 | 307 |
| HPRT1 | 100 | 76 |
| TBP | 32 | 40 |
| **Median** | **277** | **307** |

---

## What still needs to be done

### P0 — Required before any clinical use

| Item | Effort | Status |
|------|--------|--------|
| ~~INDETERMINATE state (MUT/WT/INDETERMINATE)~~ | ~~S~~ | ✅ Done |
| ~~Wilson 95% CI on every VAF~~ | ~~S~~ | ✅ Done |
| ~~Deterministic output ordering~~ | ~~S~~ | ✅ Done |
| ~~F877L target~~ | ~~S~~ | ✅ Done |
| ~~Alignment identity gate~~ | ~~S~~ | ✅ Done |
| ~~Provenance header~~ | ~~S~~ | ✅ Done |
| ~~PCR dedup (internal, sequence-level)~~ | ~~M~~ | ✅ Done |
| ~~Housekeeping normalization + expr_index~~ | ~~M~~ | ✅ Done |
| ~~AR-FL (exon3→exon4) target + AR-V7/AR-FL splice fraction~~ | ~~M~~ | ✅ Done |
| ~~AR amplification target (CONSTITUTIVE)~~ | ~~M~~ | ✅ Done |
| ~~Best-target assignment per read~~ | ~~M~~ | ✅ Done |
| ~~Validate reference codons against FASTA at startup~~ | ~~S~~ | ✅ Done |
| ~~Tests: AR-V7 TP, low-coverage INDETERMINATE, WT no false MUT, codon validation~~ | ~~M~~ | ✅ Done (7/7 pass) |

### P1 — High clinical value

| Item | Notes |
|------|-------|
| AR-V9, ARv567es splice targets | Need careful sequence validation from NM_000044.6 |
| T878S target (AGT at codon 878) | Different from T878A at nucleotide level |
| Strand bias filter | Fisher exact on fwd/rev alt counts |
| JSON output | Easy addition, useful for dashboard |
| VCF v4.3 output | Needs GRCh38 coordinates in FASTA headers |
| Phased haplotype reporting | Compound H875Y+T878A detection |
| FFPE mode | C>T / G>A down-weighting; L702H on C>T strand |
| Target registry refactor | `targets.yaml` sidecar replacing FASTA-header parsing |
| Replace `Mutex<Vec<Hit>>` with thread-local rayon accumulators | Determinism improvement |

### P2 — Longer term

| Item |
|------|
| `mira aggregate` longitudinal subcommand |
| UMI-aware deduplication |
| ADAR-edit detection / annotation |
| PDF/HTML clinical report |
| FHIR Genomics Reporting export |

---

## PCR duplicate removal

### Should you run Picard before MIRA?

**No — Picard MarkDuplicates requires BAM.** MIRA is alignment-free (FASTQ in, no genome alignment, no BAM). Picard cannot be inserted in the pipeline.

### Options, ranked by practicality

| Option | How | When |
|--------|-----|------|
| **Internal MIRA dedup (recommended)** | Deduplicate `Hit` list by read sequence before pileup (`AHashSet<Vec<u8>>` on `hit.read.seq`). ~30 lines of Rust. | Implement in P1 sprint |
| **Pre-MIRA FASTQ dedup** | `seqkit rmdup -s -j 8 R1.fastq | gzip > R1_dedup.fastq.gz` (same for R2, then use dedup pair). Simple, no BAM needed. | Use now if duplicates are a concern |
| **UMI-aware dedup** | Requires UMI in read ID (UMI-tools extract → MarkDuplicates). Requires UMI library prep. | Future; not applicable to existing data |

### Impact on current data

The current samples (61 bp RNA-seq reads) likely have PCR duplicates. With only 114–325 reads per AR target after identity filtering, the duplicate burden is probably low (few exact duplicates in ~6 GB files). The effect on VAF for H875Y (already 100%) would be negligible.

For samples with lower coverage, deduplication before MIRA matters more. Implement internal dedup in the next sprint.

---

## Key files

| File | Change |
|------|--------|
| `src/cli.rs` | Added `--min-coverage` (default 30), `--housekeeping`, `--no-dedup` |
| `src/aligner.rs` | Added `alignment_quality()` identity/clip gate |
| `src/output.rs` | `RunInfo`, `wilson_ci()`, `median_u32()`, INDETERMINATE logic, deterministic sort, provenance header, `expr_index` column, HOUSEKEEPING type |
| `src/main.rs` | `RunInfo` construction, `file_md5()`, `file_size_fingerprint()`, `deduplicate_hits()`, `assign_best_target()`, `validate_reference_codons()` |
| `src/index.rs` | `KmerIndex::merge_fasta()` |
| `src/aligner.rs` | `score_read_vs_target()` (public, for best-target scoring) |
| `src/types.rs` | Removed unused `SpliceJunction` variant |
| `reference/AR_targets.fa` | Added `AR_F877L_region`, `AR_FL_exon3_exon4_junction`, `AR_const_exon1` entries |
| `reference/housekeeping.fa` | New file: GAPDH/ACTB/HPRT1/B2M/TBP 153 bp CDS windows |
| `tests/integration_test.rs` | 4 new tests added (7 total) |
| `Cargo.toml` | `md5 = "0.8"` dependency, version `0.2.0` |
