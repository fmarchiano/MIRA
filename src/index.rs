use ahash::{AHashMap, AHashSet};
use anyhow::{Context, Result};
use needletail::parse_fastx_file;

use crate::types::{Target, TargetId};

pub struct KmerIndex {
    pub k: usize,
    pub map: AHashMap<u64, Vec<TargetId>>,
    pub targets: Vec<Target>,
}

impl KmerIndex {
    pub fn build(fasta_path: &std::path::Path, k: usize) -> Result<Self> {
        let mut targets: Vec<Target> = Vec::new();
        let mut map: AHashMap<u64, Vec<TargetId>> = AHashMap::new();

        let mut reader = parse_fastx_file(fasta_path)
            .with_context(|| format!("Failed to open reference FASTA: {}", fasta_path.display()))?;

        while let Some(record) = reader.next() {
            let rec = record.with_context(|| "Error parsing reference FASTA record")?;
            let id = targets.len();
            let seq = rec.seq().to_vec();
            let name = String::from_utf8_lossy(rec.id()).to_string();

            if seq.len() >= k {
                for i in 0..=(seq.len() - k) {
                    let kmer = &seq[i..i + k];
                    if contains_nonacgt(kmer) {
                        continue;
                    }
                    let h = canonical_hash(kmer);
                    let entry = map.entry(h).or_default();
                    if !entry.contains(&id) {
                        entry.push(id);
                    }
                }
            }

            targets.push(Target { id, name, seq });
        }

        anyhow::ensure!(!targets.is_empty(), "Reference FASTA contains no sequences");

        Ok(KmerIndex { k, map, targets })
    }

    /// Append targets from a second FASTA into an existing index.
    /// Target IDs continue from where the primary index left off.
    pub fn merge_fasta(index: &mut Self, fasta_path: &std::path::Path, k: usize) -> Result<()> {
        let mut reader = parse_fastx_file(fasta_path)
            .with_context(|| format!("Failed to open housekeeping FASTA: {}", fasta_path.display()))?;

        while let Some(record) = reader.next() {
            let rec = record.with_context(|| "Error parsing housekeeping FASTA record")?;
            let id = index.targets.len();
            let seq = rec.seq().to_vec();
            let name = String::from_utf8_lossy(rec.id()).to_string();

            if seq.len() >= k {
                for i in 0..=(seq.len() - k) {
                    let kmer = &seq[i..i + k];
                    if contains_nonacgt(kmer) {
                        continue;
                    }
                    let h = canonical_hash(kmer);
                    let entry = index.map.entry(h).or_default();
                    if !entry.contains(&id) {
                        entry.push(id);
                    }
                }
            }

            index.targets.push(Target { id, name, seq });
        }

        Ok(())
    }

    /// Fast pre-filter: true if any k-mer in seq has an exact hit.
    pub fn has_exact_hit(&self, seq: &[u8]) -> bool {
        let k = self.k;
        if seq.len() < k {
            return false;
        }
        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];
            if contains_nonacgt(kmer) {
                continue;
            }
            if self.map.contains_key(&canonical_hash(kmer)) {
                return true;
            }
        }
        false
    }

    /// Check if a read (or any of its mismatch neighbors) hits the index.
    /// Returns set of target IDs hit.
    pub fn scan_read(&self, seq: &[u8], max_mismatches: usize) -> Vec<TargetId> {
        let k = self.k;
        if seq.len() < k {
            return vec![];
        }

        let mut seen: AHashSet<TargetId> = AHashSet::new();
        let mut hits: Vec<TargetId> = Vec::new();

        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];
            if contains_nonacgt(kmer) {
                continue;
            }

            // Exact hit
            if let Some(ids) = self.map.get(&canonical_hash(kmer)) {
                add_hits(ids, &mut seen, &mut hits);
                continue;
            }

            if max_mismatches == 0 {
                continue;
            }

            // 1-mismatch neighbors
            let mut nbr = kmer.to_vec();
            for pos in 0..k {
                let orig = nbr[pos];
                for &alt in b"ACGT" {
                    if alt == orig {
                        continue;
                    }
                    nbr[pos] = alt;
                    if let Some(ids) = self.map.get(&canonical_hash(&nbr)) {
                        add_hits(ids, &mut seen, &mut hits);
                    }
                    nbr[pos] = orig;
                }
            }

            if max_mismatches < 2 {
                continue;
            }

            // 2-mismatch neighbors — independent of whether any 1-mismatch hit was found
            for pos in 0..k {
                let orig1 = nbr[pos];
                for &alt1 in b"ACGT" {
                    if alt1 == orig1 {
                        continue;
                    }
                    nbr[pos] = alt1;
                    for pos2 in (pos + 1)..k {
                        let orig2 = nbr[pos2];
                        for &alt2 in b"ACGT" {
                            if alt2 == orig2 {
                                continue;
                            }
                            nbr[pos2] = alt2;
                            if let Some(ids) = self.map.get(&canonical_hash(&nbr)) {
                                add_hits(ids, &mut seen, &mut hits);
                            }
                            nbr[pos2] = orig2;
                        }
                    }
                    nbr[pos] = orig1;
                }
            }
        }

        hits
    }
}

fn add_hits(ids: &[TargetId], seen: &mut AHashSet<TargetId>, hits: &mut Vec<TargetId>) {
    for &tid in ids {
        if seen.insert(tid) {
            hits.push(tid);
        }
    }
}

fn contains_nonacgt(kmer: &[u8]) -> bool {
    kmer.iter()
        .any(|&b| !matches!(b.to_ascii_uppercase(), b'A' | b'C' | b'G' | b'T'))
}

fn canonical_hash(kmer: &[u8]) -> u64 {
    let fwd = hash_kmer_fwd(kmer);
    let rc = hash_kmer_rc(kmer);
    fwd.min(rc)
}

fn hash_kmer_fwd(kmer: &[u8]) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for &b in kmer {
        h ^= base_encode(b) as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn hash_kmer_rc(kmer: &[u8]) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for &b in kmer.iter().rev() {
        h ^= base_encode(complement(b)) as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn base_encode(b: u8) -> u8 {
    match b.to_ascii_uppercase() {
        b'A' => 0,
        b'C' => 1,
        b'G' => 2,
        b'T' => 3,
        _ => 4, // N/ambiguous — distinct from all ACGT values
    }
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
