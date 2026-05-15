use ahash::AHashMap;
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
                    let h = canonical_hash(kmer);
                    map.entry(h).or_default().push(id);
                }
            }

            targets.push(Target { id, name, seq });
        }

        anyhow::ensure!(!targets.is_empty(), "Reference FASTA contains no sequences");

        Ok(KmerIndex { k, map, targets })
    }

    /// Fast pre-filter: true if any k-mer in seq has an exact hit.
    pub fn has_exact_hit(&self, seq: &[u8]) -> bool {
        let k = self.k;
        if seq.len() < k {
            return false;
        }
        for i in 0..=(seq.len() - k) {
            if self.map.contains_key(&canonical_hash(&seq[i..i + k])) {
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

        let mut hits: Vec<TargetId> = Vec::new();

        for i in 0..=(seq.len() - k) {
            let kmer = &seq[i..i + k];

            // exact hit
            let h = canonical_hash(kmer);
            if let Some(ids) = self.map.get(&h) {
                for &tid in ids {
                    if !hits.contains(&tid) {
                        hits.push(tid);
                    }
                }
                continue; // exact match found, skip mismatch probing for this position
            }

            if max_mismatches == 0 {
                continue;
            }

            // mismatch neighbors: mutate each position to the 3 other bases
            let mut neighbor = kmer.to_vec();
            'outer: for pos in 0..k {
                let orig = neighbor[pos];
                for &alt in b"ACGT" {
                    if alt == orig {
                        continue;
                    }
                    neighbor[pos] = alt;
                    let nh = canonical_hash(&neighbor);
                    if let Some(ids) = self.map.get(&nh) {
                        for &tid in ids {
                            if !hits.contains(&tid) {
                                hits.push(tid);
                            }
                        }
                        neighbor[pos] = orig;
                        // found with 1 mismatch — for max_mismatches==1 this is sufficient per position
                        if max_mismatches <= 1 {
                            continue;
                        }
                        // for d=2: probe second mismatch position
                        for pos2 in (pos + 1)..k {
                            let orig2 = neighbor[pos2];
                            for &alt2 in b"ACGT" {
                                if alt2 == orig2 {
                                    continue;
                                }
                                neighbor[pos2] = alt2;
                                let nh2 = canonical_hash(&neighbor);
                                if let Some(ids2) = self.map.get(&nh2) {
                                    for &tid in ids2 {
                                        if !hits.contains(&tid) {
                                            hits.push(tid);
                                        }
                                    }
                                }
                                neighbor[pos2] = orig2;
                            }
                        }
                        if !hits.is_empty() {
                            break 'outer;
                        }
                    }
                }
                neighbor[pos] = orig;
            }
        }

        hits
    }
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
        _ => 0,
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
