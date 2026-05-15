use ahash::AHashMap;

pub type TargetId = usize;

#[derive(Debug, Clone)]
pub struct Target {
    pub id: TargetId,
    pub name: String,
    pub seq: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Read {
    pub id: Vec<u8>,
    pub seq: Vec<u8>,
    pub qual: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub read: Read,
    pub target_id: TargetId,
}

/// Per-position pileup counts for a single target
#[derive(Debug, Default, Clone)]
pub struct PileupColumn {
    pub ref_count: u32,
    pub alt_counts: AHashMap<u8, u32>,
    pub del_count: u32,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct Pileup {
    pub target_id: TargetId,
    pub columns: Vec<PileupColumn>,
    pub total_reads: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantType {
    Snp,
    Indel,
    SpliceJunction,
    Presence,
}

impl std::fmt::Display for VariantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariantType::Snp => write!(f, "SNP"),
            VariantType::Indel => write!(f, "INDEL"),
            VariantType::SpliceJunction => write!(f, "SPLICE_JUNCTION"),
            VariantType::Presence => write!(f, "PRESENCE"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub target_id: TargetId,
    pub variant_type: VariantType,
    pub position: u32,
    pub ref_allele: String,
    pub alt_allele: String,
    pub supporting_reads: u32,
    pub total_reads: u32,
}

impl Variant {
    pub fn frequency(&self) -> f64 {
        if self.total_reads == 0 {
            0.0
        } else {
            self.supporting_reads as f64 / self.total_reads as f64
        }
    }
}
