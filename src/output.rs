use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::types::{Target, Variant, VariantType};

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

pub struct RunInfo {
    pub mira_version: &'static str,
    pub reference: String,
    pub ref_md5: String,
    pub r1: String,
    pub r1_md5: String,
    pub r2: Option<String>,
    pub r2_md5: Option<String>,
    pub params: String,
    pub timestamp: u64,
}

impl RunInfo {
    fn header_line(&self) -> String {
        let r2_part = match (&self.r2, &self.r2_md5) {
            (Some(r2), Some(md5)) => format!(" r2={r2} r2_md5={md5}"),
            _ => String::new(),
        };
        format!(
            "# mira={} ref={} ref_md5={} r1={} r1_md5={}{} params=[{}] timestamp={}",
            self.mira_version,
            self.reference,
            self.ref_md5,
            self.r1,
            self.r1_md5,
            r2_part,
            self.params,
            self.timestamp,
        )
    }
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Wilson 95% confidence interval for a proportion.
/// Returns (lo, hi) clamped to [0, 1]. Returns (0.0, 1.0) when n == 0.
fn wilson_ci(k: u32, n: u32) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let p = k as f64 / n as f64;
    let z = 1.96_f64;
    let z2 = z * z;
    let nf = n as f64;
    let center = p + z2 / (2.0 * nf);
    let margin = z * (p * (1.0 - p) / nf + z2 / (4.0 * nf * nf)).sqrt();
    let denom = 1.0 + z2 / nf;
    let lo = ((center - margin) / denom).max(0.0);
    let hi = ((center + margin) / denom).min(1.0);
    (lo, hi)
}

fn median_u32(vals: &[u32]) -> u32 {
    if vals.is_empty() {
        return 0;
    }
    let mut sorted = vals.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    } else {
        sorted[n / 2]
    }
}

// ---------------------------------------------------------------------------
// Raw pileup TSV
// ---------------------------------------------------------------------------

pub fn write_tsv(
    path: &Path,
    sample: &str,
    variants: &[Variant],
    targets: &[Target],
    info: &RunInfo,
) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);

    writeln!(w, "{}", info.header_line())?;
    writeln!(w, "sample\ttarget_id\tvariant_type\tposition\tref\talt\tsupporting_reads\ttotal_reads\tfrequency")?;

    // Deterministic order: target_id → PRESENCE last → position → alt allele
    let mut sorted: Vec<&Variant> = variants.iter().collect();
    sorted.sort_by(|a, b| {
        let a_pres = (a.variant_type == VariantType::Presence) as u8;
        let b_pres = (b.variant_type == VariantType::Presence) as u8;
        a.target_id.cmp(&b.target_id)
            .then(a_pres.cmp(&b_pres))  // PRESENCE rows sort after positional variants
            .then(a.position.cmp(&b.position))
            .then(a.alt_allele.cmp(&b.alt_allele))
    });

    for v in sorted {
        let target_name = &targets[v.target_id].name;
        let pos = if v.variant_type == VariantType::Presence {
            ".".to_string()
        } else {
            v.position.to_string()
        };
        writeln!(
            w,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}",
            sample,
            target_name,
            v.variant_type,
            pos,
            v.ref_allele,
            v.alt_allele,
            v.supporting_reads,
            v.total_reads,
            v.frequency(),
        )?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Target metadata parsing
// ---------------------------------------------------------------------------

pub struct TargetMeta {
    pub short_name: String,
    pub target_type: String,
    pub variant_offset: Option<u32>,
    pub ref_codon: Option<[u8; 3]>,
    pub alt_codon: Option<[u8; 3]>,
}

fn parse_codon(s: &str, key: &str) -> Option<[u8; 3]> {
    let pos = s.find(key)?;
    let after = &s[pos + key.len()..];
    let bases: Vec<u8> = after
        .chars()
        .take_while(|c| matches!(c, 'A' | 'C' | 'G' | 'T'))
        .map(|c| c.to_ascii_uppercase() as u8)
        .collect();
    if bases.len() >= 3 {
        Some([bases[0], bases[1], bases[2]])
    } else {
        None
    }
}

pub fn parse_target_meta(name: &str) -> TargetMeta {
    let short_name = name.split_whitespace().next().unwrap_or(name).to_string();

    let target_type = if name.contains("variant_type=SPLICE_JUNCTION") {
        "SPLICE_JUNCTION"
    } else if name.contains("variant_type=SNP") {
        "SNP"
    } else if name.contains("variant_type=HOUSEKEEPING") {
        "HOUSEKEEPING"
    } else if name.contains("variant_type=CONSTITUTIVE") {
        "CONSTITUTIVE"
    } else {
        eprintln!("[mira] WARNING: target '{}' has no variant_type= annotation — classified as UNKNOWN", short_name);
        "UNKNOWN"
    }
    .to_string();

    let variant_offset = name.find("offset").and_then(|pos| {
        let after = name[pos + 6..].trim_start_matches('=');
        after
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
    });

    // Only accept unambiguous _ref= / _alt= keys to avoid partial matches in other fields
    let ref_codon = parse_codon(name, "_ref=");
    let alt_codon = parse_codon(name, "_alt=");

    TargetMeta {
        short_name,
        target_type,
        variant_offset,
        ref_codon,
        alt_codon,
    }
}

// ---------------------------------------------------------------------------
// Novel variants TSV
// ---------------------------------------------------------------------------

pub fn write_novel(
    path: &Path,
    sample: &str,
    variants: &[Variant],
    targets: &[Target],
    novel_min_vaf: f64,
    info: &RunInfo,
) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);
    writeln!(w, "{}", info.header_line())?;
    writeln!(w, "sample\ttarget\tposition\tref\talt\tsupporting_reads\ttotal_reads\tvaf")?;

    let mut novel: Vec<&Variant> = variants
        .iter()
        .filter(|v| {
            if v.variant_type != VariantType::Snp {
                return false;
            }
            if v.frequency() < novel_min_vaf {
                return false;
            }
            let target = &targets[v.target_id];
            let meta = parse_target_meta(&target.name);
            if meta.target_type == "SNP" {
                if let (Some(offset), Some(ref_cod), Some(alt_cod)) =
                    (meta.variant_offset, meta.ref_codon, meta.alt_codon)
                {
                    for i in 0..3usize {
                        if ref_cod[i] == alt_cod[i] {
                            continue;
                        }
                        let known_pos = offset + 1 + i as u32;
                        let known_alt = alt_cod[i];
                        if v.position == known_pos
                            && v.alt_allele.as_bytes().first().copied() == Some(known_alt)
                        {
                            return false; // known expected mutation
                        }
                    }
                }
            }
            true
        })
        .collect();

    // Deterministic order
    novel.sort_by(|a, b| {
        a.target_id.cmp(&b.target_id)
            .then(a.position.cmp(&b.position))
            .then(a.alt_allele.cmp(&b.alt_allele))
    });

    for v in novel {
        let target = &targets[v.target_id];
        let meta = parse_target_meta(&target.name);
        writeln!(
            w,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.4}",
            sample,
            meta.short_name,
            v.position,
            v.ref_allele,
            v.alt_allele,
            v.supporting_reads,
            v.total_reads,
            v.frequency(),
        )?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Summary TSV
// ---------------------------------------------------------------------------

pub fn write_summary(
    path: &Path,
    sample: &str,
    variants: &[Variant],
    targets: &[Target],
    min_reads: u32,
    min_mut_vaf: f64,
    min_coverage: u32,
    info: &RunInfo,
) -> Result<()> {
    let mut by_target: HashMap<usize, Vec<&Variant>> = HashMap::new();
    for v in variants {
        by_target.entry(v.target_id).or_default().push(v);
    }

    let target_total_reads = |target: &Target| -> u32 {
        let tvs = by_target.get(&target.id).map(|v| v.as_slice()).unwrap_or(&[]);
        tvs.iter()
            .filter(|v| v.variant_type == VariantType::Presence)
            .map(|v| v.total_reads)
            .next()
            .unwrap_or_else(|| tvs.iter().map(|v| v.total_reads).max().unwrap_or(0))
    };

    // Collect housekeeping read counts for expression normalization
    let hk_reads: Vec<u32> = targets
        .iter()
        .filter(|t| parse_target_meta(&t.name).target_type == "HOUSEKEEPING")
        .map(|t| target_total_reads(t))
        .collect();
    let hk_median = median_u32(&hk_reads);

    // Pre-compute AR-V7 / AR-FL splice fraction:
    // numerator = reads for the target whose name contains "exon3_CE3" or "_V7"
    // denominator = numerator + reads for target whose name contains "exon3_exon4" or "_FL"
    let splice_reads = |substr: &str| -> u32 {
        targets
            .iter()
            .filter(|t| {
                let n = &t.name;
                n.contains("variant_type=SPLICE_JUNCTION") && n.contains(substr)
            })
            .map(|t| target_total_reads(t))
            .next()
            .unwrap_or(0)
    };
    let v7_reads = splice_reads("CE3_junction").max(splice_reads("V7_exon3"));
    let fl_reads = splice_reads("exon3_exon4");
    let v7_fl_denom = v7_reads + fl_reads;

    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);
    writeln!(w, "{}", info.header_line())?;
    writeln!(w, "sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf\tvaf_ci_lo\tvaf_ci_hi\tsplice_fraction\texpr_index")?;

    // Targets are iterated in index order — deterministic
    for target in targets {
        let meta = parse_target_meta(&target.name);
        let target_variants = by_target.get(&target.id).map(|v| v.as_slice()).unwrap_or(&[]);

        // Use PRESENCE row total_reads as the per-target coverage metric
        let total_reads = target_variants
            .iter()
            .filter(|v| v.variant_type == VariantType::Presence)
            .map(|v| v.total_reads)
            .next()
            .unwrap_or_else(|| target_variants.iter().map(|v| v.total_reads).max().unwrap_or(0));

        // Expression index: reads / HK median (for non-HK targets; HK targets emit ".")
        let expr_index = if meta.target_type == "HOUSEKEEPING" {
            ".".to_string()
        } else if hk_median > 0 {
            format!("{:.4}", total_reads as f64 / hk_median as f64)
        } else {
            ".".to_string()
        };

        // Splice fraction: AR-V7 / (AR-V7 + AR-FL). Only emitted on the V7 junction row.
        let is_v7_row = meta.target_type == "SPLICE_JUNCTION"
            && (target.name.contains("CE3_junction") || target.name.contains("V7_exon3"));
        let splice_fraction = if is_v7_row && v7_fl_denom > 0 {
            format!("{:.4}", v7_reads as f64 / v7_fl_denom as f64)
        } else {
            ".".to_string()
        };

        let (call, alt_reads, vaf, ci_lo, ci_hi) = match meta.target_type.as_str() {
            "SPLICE_JUNCTION" => {
                if total_reads >= min_reads {
                    ("PRESENCE".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string())
                } else if total_reads >= min_coverage {
                    ("ABSENCE".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string())
                } else {
                    ("INDETERMINATE".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string())
                }
            }
            "SNP" => {
                // pos_depth = per-position depth at the SNP site; used for VAF and CI
                let mut mut_call: Option<(u32, u32, f64)> = None;
                if let (Some(offset), Some(ref_cod), Some(alt_cod)) =
                    (meta.variant_offset, meta.ref_codon, meta.alt_codon)
                {
                    'search: for i in 0..3usize {
                        if ref_cod[i] == alt_cod[i] {
                            continue;
                        }
                        let check_pos = offset + 1 + i as u32;
                        let expected_alt = alt_cod[i];
                        for v in target_variants {
                            if v.variant_type == VariantType::Snp
                                && v.position == check_pos
                                && v.alt_allele.as_bytes().first().copied() == Some(expected_alt)
                                && v.frequency() >= min_mut_vaf
                            {
                                mut_call = Some((v.supporting_reads, v.total_reads, v.frequency()));
                                break 'search;
                            }
                        }
                    }
                }
                match mut_call {
                    Some((reads, pos_depth, freq)) => {
                        let (lo, hi) = wilson_ci(reads, pos_depth);
                        (
                            "MUT".to_string(),
                            reads.to_string(),
                            format!("{:.4}", freq),
                            format!("{:.4}", lo),
                            format!("{:.4}", hi),
                        )
                    }
                    None => {
                        if total_reads >= min_coverage {
                            ("WT".to_string(), "0".to_string(), "0.0000".to_string(), ".".to_string(), ".".to_string())
                        } else {
                            ("INDETERMINATE".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string())
                        }
                    }
                }
            }
            "HOUSEKEEPING" | "CONSTITUTIVE" => {
                if total_reads >= min_reads {
                    ("EXPRESSED".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string())
                } else {
                    ("INDETERMINATE".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string())
                }
            }
            _ => ("UNKNOWN".to_string(), ".".to_string(), ".".to_string(), ".".to_string(), ".".to_string()),
        };

        writeln!(
            w,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            sample, meta.short_name, meta.target_type, call, total_reads, alt_reads, vaf, ci_lo, ci_hi, splice_fraction, expr_index
        )?;
    }

    Ok(())
}
