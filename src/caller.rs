use crate::types::{Pileup, Target, Variant, VariantType};

pub fn call_variants(
    pileups: &[Pileup],
    targets: &[Target],
    min_reads: u32,
) -> Vec<Variant> {
    let mut variants: Vec<Variant> = Vec::new();

    for pileup in pileups {
        let target = &targets[pileup.target_id];

        for (pos, col) in pileup.columns.iter().enumerate() {
            // Per-position depth is the correct VAF denominator
            let pos_depth = col.total;

            // SNPs
            for (&alt_base, &count) in &col.alt_counts {
                if count >= min_reads {
                    variants.push(Variant {
                        target_id: pileup.target_id,
                        variant_type: VariantType::Snp,
                        position: pos as u32 + 1,
                        ref_allele: (target.seq[pos].to_ascii_uppercase() as char).to_string(),
                        alt_allele: (alt_base as char).to_string(),
                        supporting_reads: count,
                        total_reads: pos_depth,
                    });
                }
            }

            // Deletions
            if col.del_count >= min_reads {
                variants.push(Variant {
                    target_id: pileup.target_id,
                    variant_type: VariantType::Indel,
                    position: pos as u32 + 1,
                    ref_allele: (target.seq[pos].to_ascii_uppercase() as char).to_string(),
                    alt_allele: "-".to_string(),
                    supporting_reads: col.del_count,
                    total_reads: pos_depth,
                });
            }
        }

        // Always emit PRESENCE row — provides consistent per-target coverage record
        variants.push(Variant {
            target_id: pileup.target_id,
            variant_type: VariantType::Presence,
            position: 0,
            ref_allele: ".".to_string(),
            alt_allele: ".".to_string(),
            supporting_reads: pileup.total_reads,
            total_reads: pileup.total_reads,
        });
    }

    variants
}
