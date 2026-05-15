use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::types::{Target, Variant, VariantType};

pub fn write_tsv(
    path: &Path,
    sample: &str,
    variants: &[Variant],
    targets: &[Target],
) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);

    writeln!(w, "sample\ttarget_id\tvariant_type\tposition\tref\talt\tsupporting_reads\ttotal_reads\tfrequency")?;

    for v in variants {
        let target_name = &targets[v.target_id].name;
        let pos = if v.variant_type == VariantType::Presence { ".".to_string() } else { v.position.to_string() };
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

struct TargetMeta {
    short_name: String,
    target_type: String,
    variant_offset: Option<u32>,
    ref_codon: Option<[u8; 3]>,
    alt_codon: Option<[u8; 3]>,
}

fn parse_codon(s: &str, key: &str) -> Option<[u8; 3]> {
    let pos = s.find(key)?;
    let after = &s[pos + key.len()..];
    let bases: Vec<u8> = after
        .chars()
        .take_while(|c| matches!(c, 'A' | 'C' | 'G' | 'T'))
        .map(|c| c.to_ascii_uppercase() as u8)
        .collect();
    if bases.len() >= 3 { Some([bases[0], bases[1], bases[2]]) } else { None }
}

fn parse_target_meta(name: &str) -> TargetMeta {
    let short_name = name.split_whitespace().next().unwrap_or(name).to_string();

    let target_type = if name.contains("variant_type=SPLICE_JUNCTION") {
        "SPLICE_JUNCTION"
    } else if name.contains("variant_type=SNP") {
        "SNP"
    } else {
        "UNKNOWN"
    }.to_string();

    let variant_offset = name.find("offset").and_then(|pos| {
        let after = name[pos + 6..].trim_start_matches('=');
        after.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u32>().ok()
    });

    let ref_codon = parse_codon(name, "_ref=").or_else(|| parse_codon(name, "ref="));
    let alt_codon = parse_codon(name, "_alt=").or_else(|| parse_codon(name, "alt="));

    TargetMeta { short_name, target_type, variant_offset, ref_codon, alt_codon }
}

pub fn write_novel(
    path: &Path,
    sample: &str,
    variants: &[Variant],
    targets: &[Target],
    novel_min_vaf: f64,
) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);
    writeln!(w, "sample\ttarget\tposition\tref\talt\tsupporting_reads\ttotal_reads\tvaf")?;

    for v in variants {
        if v.variant_type != VariantType::Snp {
            continue;
        }
        if v.frequency() < novel_min_vaf {
            continue;
        }

        let target = &targets[v.target_id];
        let meta = parse_target_meta(&target.name);

        // For SNP targets: skip if this is the expected known mutation
        if meta.target_type == "SNP" {
            if let (Some(offset), Some(ref_cod), Some(alt_cod)) =
                (meta.variant_offset, meta.ref_codon, meta.alt_codon)
            {
                let mut is_known = false;
                for i in 0..3usize {
                    if ref_cod[i] == alt_cod[i] { continue; }
                    let known_pos = offset + 1 + i as u32;
                    let known_alt = alt_cod[i];
                    if v.position == known_pos
                        && v.alt_allele.as_bytes().first().copied() == Some(known_alt)
                    {
                        is_known = true;
                        break;
                    }
                }
                if is_known { continue; }
            }
        }

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

pub fn write_summary(
    path: &Path,
    sample: &str,
    variants: &[Variant],
    targets: &[Target],
    min_reads: u32,
    min_mut_vaf: f64,
) -> Result<()> {
    let mut by_target: HashMap<usize, Vec<&Variant>> = HashMap::new();
    for v in variants {
        by_target.entry(v.target_id).or_default().push(v);
    }

    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);
    writeln!(w, "sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf")?;

    for target in targets {
        let meta = parse_target_meta(&target.name);
        let target_variants = by_target.get(&target.id).map(|v| v.as_slice()).unwrap_or(&[]);
        let total_reads = target_variants.iter().map(|v| v.total_reads).max().unwrap_or(0);

        let (call, alt_reads, vaf) = match meta.target_type.as_str() {
            "SPLICE_JUNCTION" => {
                if total_reads >= min_reads {
                    ("PRESENCE".to_string(), ".".to_string(), ".".to_string())
                } else {
                    ("ABSENCE".to_string(), ".".to_string(), ".".to_string())
                }
            }
            "SNP" => {
                let mut mut_call: Option<(u32, f64)> = None;
                if let (Some(offset), Some(ref_cod), Some(alt_cod)) =
                    (meta.variant_offset, meta.ref_codon, meta.alt_codon)
                {
                    'search: for i in 0..3usize {
                        if ref_cod[i] == alt_cod[i] { continue; }
                        let check_pos = offset + 1 + i as u32;
                        let expected_alt = alt_cod[i];
                        for v in target_variants {
                            if v.variant_type == VariantType::Snp
                                && v.position == check_pos
                                && v.alt_allele.as_bytes().first().copied() == Some(expected_alt)
                                && v.frequency() >= min_mut_vaf
                            {
                                mut_call = Some((v.supporting_reads, v.frequency()));
                                break 'search;
                            }
                        }
                    }
                }
                match mut_call {
                    Some((reads, freq)) => (
                        "MUT".to_string(),
                        reads.to_string(),
                        format!("{:.4}", freq),
                    ),
                    None => ("WT".to_string(), "0".to_string(), "0.0000".to_string()),
                }
            }
            _ => ("UNKNOWN".to_string(), ".".to_string(), ".".to_string()),
        };

        writeln!(w, "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            sample, meta.short_name, meta.target_type, call, total_reads, alt_reads, vaf)?;
    }

    Ok(())
}
