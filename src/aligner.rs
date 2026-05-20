use bio::alignment::pairwise::{Aligner, Scoring};
use anyhow::Result;

use crate::types::{Hit, Pileup, PileupColumn, Target};

/// Return the best SW alignment score for a read against a target (tries fwd and RC).
pub fn score_read_vs_target(seq: &[u8], target_seq: &[u8]) -> i32 {
    align_best_orientation(seq, &[], target_seq).0.score
}

pub fn build_pileups(
    hits: &[Hit],
    targets: &[Target],
    min_base_qual: u8,
) -> Result<Vec<Pileup>> {
    let mut pileups: Vec<Pileup> = targets
        .iter()
        .map(|t| Pileup {
            target_id: t.id,
            columns: vec![PileupColumn::default(); t.seq.len()],
            total_reads: 0,
        })
        .collect();

    for hit in hits {
        let target = &targets[hit.target_id];
        let (alignment, oriented_seq, oriented_qual) =
            align_best_orientation(&hit.read.seq, &hit.read.qual, &target.seq);

        // Alignment quality gate: drop low-identity or heavily clipped reads.
        // Identity < 90% or soft-clip > 30% of read length indicates off-target alignment.
        let (identity, clip_frac) = alignment_quality(&alignment, oriented_seq.len());
        if identity < 0.90 || clip_frac > 0.30 {
            continue;
        }

        let pileup = &mut pileups[hit.target_id];
        pileup.total_reads += 1;

        let mut ref_pos = alignment.ystart;
        let mut read_pos = alignment.xstart;

        for op in &alignment.operations {
            use bio::alignment::AlignmentOperation::*;
            match op {
                Match | Subst => {
                    if ref_pos < target.seq.len() && read_pos < oriented_seq.len() {
                        // unwrap_or(0): qual-less reads get phred=0, always filtered
                        let base_q = oriented_qual.get(read_pos).copied().unwrap_or(0);
                        let phred = base_q.saturating_sub(33);
                        if phred >= min_base_qual {
                            let col = &mut pileup.columns[ref_pos];
                            col.total += 1;
                            let read_base = oriented_seq[read_pos].to_ascii_uppercase();
                            let ref_base = target.seq[ref_pos].to_ascii_uppercase();
                            if read_base == ref_base {
                                col.ref_count += 1;
                            } else {
                                *col.alt_counts.entry(read_base).or_insert(0) += 1;
                            }
                        }
                        ref_pos += 1;
                        read_pos += 1;
                    }
                }
                Del => {
                    if ref_pos < target.seq.len() {
                        pileup.columns[ref_pos].del_count += 1;
                        pileup.columns[ref_pos].total += 1;
                    }
                    ref_pos += 1;
                }
                Ins => {
                    read_pos += 1;
                }
                Xclip(n) | Yclip(n) => {
                    let is_x = matches!(op, Xclip(_));
                    if is_x {
                        read_pos += n;
                    } else {
                        ref_pos += n;
                    }
                }
            }
        }
    }

    Ok(pileups)
}

/// Compute (identity, soft_clip_fraction) for an alignment.
/// identity = matches / (matches + mismatches + insertions + deletions)
/// clip_frac = Xclip_bases / read_length
fn alignment_quality(aln: &bio::alignment::Alignment, read_len: usize) -> (f64, f64) {
    use bio::alignment::AlignmentOperation::*;
    let (mut matches, mut mismatches, mut insertions, mut deletions, mut soft_clips) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    for op in &aln.operations {
        match op {
            Match => matches += 1,
            Subst => mismatches += 1,
            Ins => insertions += 1,
            Del => deletions += 1,
            Xclip(n) => soft_clips += n,
            _ => {}
        }
    }
    let aligned = matches + mismatches + insertions + deletions;
    let identity = if aligned > 0 { matches as f64 / aligned as f64 } else { 0.0 };
    let clip_frac = if read_len > 0 { soft_clips as f64 / read_len as f64 } else { 1.0 };
    (identity, clip_frac)
}

/// Try forward and RC alignments; return whichever scores higher along with
/// the oriented sequence and quality array used. This handles antisense reads
/// that pass canonical k-mer matching but need RC before Smith-Waterman.
fn align_best_orientation(
    seq: &[u8],
    qual: &[u8],
    target: &[u8],
) -> (bio::alignment::Alignment, Vec<u8>, Vec<u8>) {
    let fwd = align_read(seq, target);
    let rc_seq: Vec<u8> = seq.iter().rev().map(|&b| complement_base(b)).collect();
    let rev = align_read(&rc_seq, target);
    if rev.score > fwd.score {
        let rc_qual: Vec<u8> = qual.iter().rev().copied().collect();
        (rev, rc_seq, rc_qual)
    } else {
        (fwd, seq.to_vec(), qual.to_vec())
    }
}

fn align_read(read: &[u8], target: &[u8]) -> bio::alignment::Alignment {
    let scoring = Scoring::from_scores(-4, -1, 2, -2);
    let mut aligner = Aligner::with_scoring(scoring);
    aligner.semiglobal(read, target)
}

fn complement_base(b: u8) -> u8 {
    match b.to_ascii_uppercase() {
        b'A' => b'T',
        b'T' => b'A',
        b'C' => b'G',
        b'G' => b'C',
        _ => b'N',
    }
}
