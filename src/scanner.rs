use anyhow::{Context, Result};
use needletail::parse_fastx_file;
use rayon::prelude::*;
use std::path::Path;
use std::sync::Mutex;

use crate::cli::LibraryType;
use crate::index::KmerIndex;
use crate::types::{Hit, Read, TargetId};

pub struct ScanResult {
    pub hits: Vec<Hit>,
}

pub fn scan(
    r1_path: &Path,
    r2_path: Option<&Path>,
    index: &KmerIndex,
    max_mismatches: usize,
    min_mean_qual: u8,
    library_type: &LibraryType,
) -> Result<ScanResult> {
    match r2_path {
        Some(r2) => scan_paired(r1_path, r2, index, max_mismatches, min_mean_qual, library_type),
        None => scan_single(r1_path, index, max_mismatches, min_mean_qual, library_type),
    }
}

fn scan_single(
    path: &Path,
    index: &KmerIndex,
    max_mismatches: usize,
    min_mean_qual: u8,
    library_type: &LibraryType,
) -> Result<ScanResult> {
    const BATCH: usize = 50_000;

    // Empty file fast path — avoids needletail error on 0-byte files
    if path.metadata().map(|m| m.len() == 0).unwrap_or(false) {
        return Ok(ScanResult { hits: vec![] });
    }

    let mut reader = parse_fastx_file(path)
        .with_context(|| format!("Cannot open FASTQ: {}", path.display()))?;

    let mut all_hits: Vec<Hit> = Vec::new();

    loop {
        let mut batch: Vec<Read> = Vec::with_capacity(BATCH);
        let mut done = false;
        for _ in 0..BATCH {
            match reader.next() {
                Some(rec) => {
                    let rec = rec.with_context(|| format!("Error reading FASTQ: {}", path.display()))?;
                    batch.push(Read {
                        id: rec.id().to_vec(),
                        seq: rec.seq().to_vec(),
                        qual: rec.qual().unwrap_or(&[]).to_vec(),
                    });
                }
                None => {
                    done = true;
                    break;
                }
            }
        }
        if batch.is_empty() {
            break;
        }

        let batch_hits: Mutex<Vec<Hit>> = Mutex::new(Vec::new());
        batch.par_iter().for_each(|read| {
            if !passes_mean_qual(&read.qual, min_mean_qual) {
                return;
            }
            let seq = strand_seq(read, library_type, false);
            // Pre-filter: skip reads with no exact k-mer hit in the index.
            // Safe even in mismatch mode: a read with 1–2 SNPs still has many
            // exact-matching k-mers from the mutation-free flanking region.
            if !index.has_exact_hit(&seq) {
                return;
            }
            let targets = index.scan_read(&seq, max_mismatches);
            if !targets.is_empty() {
                let mut h = batch_hits.lock().unwrap();
                for tid in targets {
                    h.push(Hit { read: read.clone(), target_id: tid });
                }
            }
        });
        all_hits.extend(batch_hits.into_inner().unwrap());

        if done {
            break;
        }
    }

    Ok(ScanResult { hits: all_hits })
}

fn scan_paired(
    r1_path: &Path,
    r2_path: &Path,
    index: &KmerIndex,
    max_mismatches: usize,
    min_mean_qual: u8,
    library_type: &LibraryType,
) -> Result<ScanResult> {
    const BATCH: usize = 50_000;

    let open_fastq = |path: &Path, label: &str| -> Result<Box<dyn needletail::FastxReader>> {
        if path.metadata().map(|m| m.len() == 0).unwrap_or(false) {
            return Err(anyhow::anyhow!("{} FASTQ is empty: {}", label, path.display()));
        }
        parse_fastx_file(path).map_err(|e| {
            anyhow::anyhow!("Cannot open {} FASTQ {}: {}", label, path.display(), e)
        })
    };

    let mut r1 = match open_fastq(r1_path, "R1") {
        Ok(r) => r,
        Err(e) if e.to_string().contains("is empty") => {
            return Ok(ScanResult { hits: vec![] });
        }
        Err(e) => return Err(e),
    };
    let mut r2 = match open_fastq(r2_path, "R2") {
        Ok(r) => r,
        Err(e) if e.to_string().contains("is empty") => {
            return Ok(ScanResult { hits: vec![] });
        }
        Err(e) => return Err(e),
    };

    let mut all_hits: Vec<Hit> = Vec::new();

    loop {
        let mut r1_batch: Vec<Read> = Vec::with_capacity(BATCH);
        let mut r2_batch: Vec<Read> = Vec::with_capacity(BATCH);
        let mut done = false;

        for _ in 0..BATCH {
            match (r1.next(), r2.next()) {
                (Some(a), Some(b)) => {
                    let a = a.with_context(|| "Error reading R1")?;
                    let b = b.with_context(|| "Error reading R2")?;
                    r1_batch.push(Read {
                        id: a.id().to_vec(),
                        seq: a.seq().to_vec(),
                        qual: a.qual().unwrap_or(&[]).to_vec(),
                    });
                    r2_batch.push(Read {
                        id: b.id().to_vec(),
                        seq: b.seq().to_vec(),
                        qual: b.qual().unwrap_or(&[]).to_vec(),
                    });
                }
                (None, None) => {
                    done = true;
                    break;
                }
                _ => anyhow::bail!("Paired-end FASTQ files have mismatched read counts"),
            }
        }
        if r1_batch.is_empty() {
            break;
        }

        let pairs: Vec<(&Read, &Read)> = r1_batch.iter().zip(r2_batch.iter()).collect();
        let batch_hits: Mutex<Vec<Hit>> = Mutex::new(Vec::new());

        pairs.par_iter().for_each(|(r1, r2)| {
            let r1_pass = passes_mean_qual(&r1.qual, min_mean_qual);
            let r2_pass = passes_mean_qual(&r2.qual, min_mean_qual);
            // Drop pair only if BOTH mates fail quality — allows mate rescue
            if !r1_pass && !r2_pass {
                return;
            }

            let s1 = strand_seq(r1, library_type, false);
            let s2 = strand_seq(r2, library_type, true);

            // Pre-filter: skip if neither read has any exact k-mer hit
            if !index.has_exact_hit(&s1) && !index.has_exact_hit(&s2) {
                return;
            }

            let t1 = index.scan_read(&s1, max_mismatches);
            let t2 = index.scan_read(&s2, max_mismatches);

            // Mate rescue: if either hits, emit both reads
            let mut all_targets: Vec<TargetId> = t1.clone();
            for t in &t2 {
                if !all_targets.contains(t) {
                    all_targets.push(*t);
                }
            }

            if !all_targets.is_empty() {
                let mut h = batch_hits.lock().unwrap();
                for &tid in &all_targets {
                    h.push(Hit { read: (*r1).clone(), target_id: tid });
                    h.push(Hit { read: (*r2).clone(), target_id: tid });
                }
            }
        });

        all_hits.extend(batch_hits.into_inner().unwrap());
        if done {
            break;
        }
    }

    Ok(ScanResult { hits: all_hits })
}

fn passes_mean_qual(qual: &[u8], threshold: u8) -> bool {
    if qual.is_empty() {
        return true;
    }
    let sum: u64 = qual.iter().map(|&q| (q.saturating_sub(33)) as u64).sum();
    let mean = sum / qual.len() as u64;
    mean >= threshold as u64
}

/// Apply strand filter: for stranded libraries, reverse-complement reads
/// that should be on the antisense strand so they can match target sense sequence.
///
/// Convention (dUTP/ligation):
///   Forward: R1 is on antisense strand → RC to get sense; R2 is sense
///   Reverse:  R1 is sense; R2 is antisense → RC R2
/// Note: this is the RF convention. STAR/Salmon call this "reverse" (dUTP).
fn strand_seq(read: &Read, library_type: &LibraryType, is_r2: bool) -> Vec<u8> {
    match library_type {
        LibraryType::Unstranded => read.seq.clone(),
        LibraryType::Forward => {
            if !is_r2 {
                rev_comp(&read.seq)
            } else {
                read.seq.clone()
            }
        }
        LibraryType::Reverse => {
            if is_r2 {
                rev_comp(&read.seq)
            } else {
                read.seq.clone()
            }
        }
    }
}

fn rev_comp(seq: &[u8]) -> Vec<u8> {
    seq.iter().rev().map(|&b| complement(b)).collect()
}

fn complement(b: u8) -> u8 {
    match b.to_ascii_uppercase() {
        b'A' => b'T',
        b'T' => b'A',
        b'C' => b'G',
        b'G' => b'C',
        _ => b'N',
    }
}
