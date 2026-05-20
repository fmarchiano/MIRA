mod aligner;
mod caller;
mod cli;
mod index;
mod output;
mod scanner;
mod types;

use ahash::{AHashMap, AHashSet};
use anyhow::Result;
use clap::Parser;
use std::path::Path;

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    if let Some(t) = args.threads {
        rayon::ThreadPoolBuilder::new().num_threads(t).build_global()?;
    }

    eprintln!("[mira] Building k-mer index from {}", args.reference.display());
    let mut kmer_index = index::KmerIndex::build(&args.reference, args.kmer_size)?;
    if let Some(ref hk_path) = args.housekeeping {
        eprintln!("[mira] Loading housekeeping reference from {}", hk_path.display());
        index::KmerIndex::merge_fasta(&mut kmer_index, hk_path, args.kmer_size)?;
    }
    eprintln!("[mira] {} targets loaded, k={}", kmer_index.targets.len(), args.kmer_size);

    validate_reference_codons(&kmer_index.targets)?;

    let r1 = &args.input[0];
    let r2 = args.input.get(1).map(|p| p.as_path());
    eprintln!("[mira] Scanning FASTQ...");
    let scan = scanner::scan(r1, r2, &kmer_index, args.max_mismatches, args.min_mean_qual, &args.library_type)?;

    let n_hits = scan.hits.len();
    if n_hits == 0 {
        eprintln!("[mira] WARNING: zero k-mer hits. Check reference FASTA and input file.");
    } else {
        eprintln!("[mira] {} read-target hits extracted", n_hits);
    }

    // Deduplicate by (target_id, read sequence) to remove PCR duplicates
    let hits = if args.no_dedup {
        scan.hits
    } else {
        let before = scan.hits.len();
        let deduped = deduplicate_hits(scan.hits);
        let removed = before - deduped.len();
        eprintln!("[mira] Dedup: {} → {} hits ({} duplicates removed)", before, deduped.len(), removed);
        deduped
    };

    // Best-target assignment: for each unique read sequence that hit multiple targets,
    // keep only the target with the highest SW alignment score.
    let hits = {
        let before = hits.len();
        let assigned = assign_best_target(hits, &kmer_index.targets);
        let reassigned = before - assigned.len();
        if reassigned > 0 {
            eprintln!("[mira] Best-target: {} reads reassigned away from lower-scoring targets", reassigned);
        }
        assigned
    };

    if let Some(ref save_path) = args.save_extracted {
        save_extracted_reads(&hits, save_path)?;
        eprintln!("[mira] Extracted reads written to {}", save_path.display());
    }

    eprintln!("[mira] Aligning and building pileups...");
    let pileups = aligner::build_pileups(&hits, &kmer_index.targets, args.min_base_qual)?;

    let variants = caller::call_variants(&pileups, &kmer_index.targets, args.min_reads);
    eprintln!("[mira] {} variant/presence rows called", variants.len());

    // Derive sample name: take everything before the first '.' in the filename
    let sample = r1
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("sample")
        .split('.')
        .next()
        .unwrap_or("sample");

    // Build provenance record.
    // MD5 only for the reference (small, critical). FASTQ files use size fingerprint
    // to avoid blocking on potentially multi-GB inputs.
    eprintln!("[mira] Computing reference checksum for provenance...");
    let ref_md5 = file_md5(&args.reference).unwrap_or_else(|_| "error".to_string());
    let r1_md5 = file_size_fingerprint(r1);
    let (r2_path, r2_md5) = if let Some(r2p) = r2 {
        (Some(r2p.display().to_string()), Some(file_size_fingerprint(r2p)))
    } else {
        (None, None)
    };
    let params = format!(
        "k={} mm={} min_reads={} min_mean_qual={} min_base_qual={} min_mut_vaf={:.2} novel_min_vaf={:.2} min_coverage={}",
        args.kmer_size, args.max_mismatches, args.min_reads,
        args.min_mean_qual, args.min_base_qual, args.min_mut_vaf,
        args.novel_min_vaf, args.min_coverage,
    );
    let info = output::RunInfo {
        mira_version: env!("CARGO_PKG_VERSION"),
        reference: args.reference.display().to_string(),
        ref_md5,
        r1: r1.display().to_string(),
        r1_md5,
        r2: r2_path,
        r2_md5,
        params,
        timestamp: unix_timestamp(),
    };

    output::write_tsv(&args.output, sample, &variants, &kmer_index.targets, &info)?;
    eprintln!("[mira] Results written to {}", args.output.display());

    let stem = args.output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();

    let summary_path = args.output.with_file_name(format!("{}.summary.tsv", stem));
    output::write_summary(
        &summary_path, sample, &variants, &kmer_index.targets,
        args.min_reads, args.min_mut_vaf, args.min_coverage, &info,
    )?;
    eprintln!("[mira] Summary written to {}", summary_path.display());

    let novel_path = args.output.with_file_name(format!("{}.novel.tsv", stem));
    output::write_novel(&novel_path, sample, &variants, &kmer_index.targets, args.novel_min_vaf, &info)?;
    eprintln!("[mira] Novel variants written to {}", novel_path.display());

    Ok(())
}

/// Verify that every SNP target's _ref= codon in the header matches the actual FASTA sequence.
/// Catches annotation typos before a run produces silently wrong calls.
fn validate_reference_codons(targets: &[types::Target]) -> Result<()> {
    for target in targets {
        let meta = output::parse_target_meta(&target.name);
        if meta.target_type != "SNP" {
            continue;
        }
        let (Some(offset), Some(ref_codon)) = (meta.variant_offset, meta.ref_codon) else {
            continue;
        };
        let offset = offset as usize;
        if offset + 3 > target.seq.len() {
            eprintln!(
                "[mira] WARNING: target '{}' offset={} out of range (seq len {})",
                meta.short_name, offset, target.seq.len()
            );
            continue;
        }
        let actual: Vec<u8> = target.seq[offset..offset + 3]
            .iter()
            .map(|b| b.to_ascii_uppercase())
            .collect();
        if actual != ref_codon {
            anyhow::bail!(
                "Reference codon mismatch in target '{}':\n  \
                 FASTA at offset={}: '{}'\n  \
                 Header _ref=: '{}'\n  \
                 Check that offset and _ref= in the FASTA header are correct.",
                meta.short_name,
                offset,
                String::from_utf8_lossy(&actual),
                String::from_utf8_lossy(&ref_codon)
            );
        }
    }
    Ok(())
}

/// For each unique read sequence that matched multiple targets, keep only the
/// target with the highest Smith-Waterman alignment score. Eliminates double-counting
/// from overlapping reference windows (e.g. AR_H875Y / AR_T878A share 144 bp).
/// For each unique read sequence that hit multiple different targets, score every candidate
/// alignment and keep only those within MIN_SCORE_GAP of the best score.
///
/// A large gap (≥ 30 ≈ 15 mismatches) indicates a clearly wrong assignment, e.g. an
/// AR-V7 junction read accidentally matching AR-FL (75 bp completely different → gap ~124).
/// A small gap (< 30) means the read cannot distinguish the targets, e.g. AR_H875Y /
/// AR_T878A share 144/153 bp — keep all so each SNP position is evaluated independently.
fn assign_best_target(hits: Vec<types::Hit>, targets: &[types::Target]) -> Vec<types::Hit> {
    const MIN_SCORE_GAP: i32 = 30;

    let mut by_seq: AHashMap<Vec<u8>, Vec<types::Hit>> = AHashMap::new();
    for h in hits {
        by_seq.entry(h.read.seq.clone()).or_default().push(h);
    }
    let mut out = Vec::new();
    let mut unique_reassignments: usize = 0;

    for (_seq, candidates) in by_seq {
        let first_tid = candidates[0].target_id;
        if !candidates.iter().any(|h| h.target_id != first_tid) {
            out.extend(candidates);
            continue;
        }
        // Score each candidate alignment
        let scored: Vec<(i32, types::Hit)> = candidates
            .into_iter()
            .map(|h| {
                let s = aligner::score_read_vs_target(&h.read.seq, &targets[h.target_id].seq);
                (s, h)
            })
            .collect();
        let max_score = scored.iter().map(|(s, _)| *s).max().unwrap_or(0);
        // Keep only candidates within MIN_SCORE_GAP of the best
        let surviving: Vec<types::Hit> = scored
            .into_iter()
            .filter(|(s, _)| max_score - s < MIN_SCORE_GAP)
            .map(|(_, h)| h)
            .collect();
        if surviving.len() == 1 {
            unique_reassignments += 1;
        }
        out.extend(surviving);
    }

    if unique_reassignments > 0 {
        eprintln!(
            "[mira] Best-target: {} reads uniquely assigned (score gap ≥ {})",
            unique_reassignments, MIN_SCORE_GAP
        );
    }
    out
}

fn deduplicate_hits(hits: Vec<types::Hit>) -> Vec<types::Hit> {
    let mut seen: AHashSet<(usize, Vec<u8>)> = AHashSet::new();
    let mut out = Vec::with_capacity(hits.len());
    for h in hits {
        if seen.insert((h.target_id, h.read.seq.clone())) {
            out.push(h);
        }
    }
    out
}

fn save_extracted_reads(hits: &[types::Hit], path: &Path) -> Result<()> {
    use std::io::Write;
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);
    let mut seen: AHashSet<Vec<u8>> = AHashSet::new();
    for hit in hits {
        if seen.insert(hit.read.id.clone()) {
            writeln!(w, "@{}", String::from_utf8_lossy(&hit.read.id))?;
            writeln!(w, "{}", String::from_utf8_lossy(&hit.read.seq))?;
            writeln!(w, "+")?;
            if hit.read.qual.is_empty() {
                writeln!(w, "{}", "I".repeat(hit.read.seq.len()))?;
            } else {
                writeln!(w, "{}", String::from_utf8_lossy(&hit.read.qual))?;
            }
        }
    }
    Ok(())
}

/// Compute MD5 of a file, streaming in 64 KiB chunks.
fn file_md5(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut ctx = md5::Context::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.consume(&buf[..n]);
    }
    Ok(format!("{:x}", ctx.finalize()))
}

/// Fast FASTQ fingerprint: "size=N bytes=B" using filesystem metadata only.
fn file_size_fingerprint(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(m) => format!("size={}", m.len()),
        Err(_) => "size=unknown".to_string(),
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
