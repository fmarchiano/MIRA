use crate::types::{Pileup, Target, Variant, VariantType};

pub fn call_variants(
    pileups: &[Pileup],
    targets: &[Target],
    min_reads: u32,
) -> Vec<Variant> {
    let mut variants: Vec<Variant> = Vec::new();

    for pileup in pileups {
        let target = &targets[pileup.target_id];
        let mut found_any = false;

        for (pos, col) in pileup.columns.iter().enumerate() {
            // SNPs
            for (&alt_base, &count) in &col.alt_counts {
                if count >= min_reads {
                    found_any = true;
                    variants.push(Variant {
                        target_id: pileup.target_id,
                        variant_type: VariantType::Snp,
                        position: pos as u32 + 1,
                        ref_allele: (target.seq[pos].to_ascii_uppercase() as char).to_string(),
                        alt_allele: (alt_base as char).to_string(),
                        supporting_reads: count,
                        total_reads: pileup.total_reads,
                    });
                }
            }

            // Deletions
            if col.del_count >= min_reads {
                found_any = true;
                variants.push(Variant {
                    target_id: pileup.target_id,
                    variant_type: VariantType::Indel,
                    position: pos as u32 + 1,
                    ref_allele: (target.seq[pos].to_ascii_uppercase() as char).to_string(),
                    alt_allele: "-".to_string(),
                    supporting_reads: col.del_count,
                    total_reads: pileup.total_reads,
                });
            }
        }

        // PRESENCE row — always emitted (supporting_reads = total aligned reads or 0)
        if pileup.total_reads > 0 {
            if !found_any {
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
        } else {
            // zero coverage — emit empty PRESENCE row per spec
            variants.push(Variant {
                target_id: pileup.target_id,
                variant_type: VariantType::Presence,
                position: 0,
                ref_allele: ".".to_string(),
                alt_allele: ".".to_string(),
                supporting_reads: 0,
                total_reads: 0,
            });
        }
    }

    variants
}
