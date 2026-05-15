use bio::alignment::pairwise::Scoring;
use bio::alignment::pairwise::banded::Aligner;
use anyhow::Result;

use crate::types::{Hit, Pileup, PileupColumn, Target};

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
        let alignment = align_read(&hit.read.seq, &target.seq);

        let pileup = &mut pileups[hit.target_id];
        pileup.total_reads += 1;

        let mut ref_pos = alignment.ystart;
        let mut read_pos = alignment.xstart;

        for op in &alignment.operations {
            use bio::alignment::AlignmentOperation::*;
            match op {
                Match | Subst => {
                    if ref_pos < target.seq.len() && read_pos < hit.read.seq.len() {
                        let base_q = hit.read.qual.get(read_pos).copied().unwrap_or(33 + 30);
                        let phred = base_q.saturating_sub(33);
                        if phred >= min_base_qual {
                            let col = &mut pileup.columns[ref_pos];
                            col.total += 1;
                            let read_base = hit.read.seq[read_pos].to_ascii_uppercase();
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
                    // soft/hard clips — advance appropriate pointer
                    let is_x = matches!(op, Xclip(_));
                    if is_x { read_pos += n; } else { ref_pos += n; }
                }
            }
        }
    }

    Ok(pileups)
}

fn align_read(read: &[u8], target: &[u8]) -> bio::alignment::Alignment {
    let scoring = Scoring::from_scores(-4, -1, 2, -2);
    let mut aligner = Aligner::with_scoring(scoring, 4, 6);
    aligner.semiglobal(read, target)
}
