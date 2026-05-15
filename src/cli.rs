use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum LibraryType {
    Unstranded,
    Forward,
    Reverse,
}

#[derive(Parser, Debug)]
#[command(name = "mira", about = "Mutation In RNA-seq Aligner — alignment-free variant detection from FASTQ")]
pub struct Cli {
    #[arg(short = 'i', long, required = true, num_args = 1..=2,
          help = "Input FASTQ file(s). One for single-end, two for paired-end (R1 R2).")]
    pub input: Vec<PathBuf>,

    #[arg(short = 'r', long, help = "Reference target sequences (FASTA)")]
    pub reference: PathBuf,

    #[arg(short = 'o', long, help = "Output TSV file")]
    pub output: PathBuf,

    #[arg(short = 'k', long, default_value_t = 31, help = "K-mer size")]
    pub kmer_size: usize,

    #[arg(short = 'm', long, default_value_t = 2, help = "Max Hamming mismatches for k-mer lookup")]
    pub max_mismatches: usize,

    #[arg(long, default_value_t = 10, help = "Min supporting reads to call a variant")]
    pub min_reads: u32,

    #[arg(long, default_value_t = 20, help = "Min mean Phred quality to keep a read")]
    pub min_mean_qual: u8,

    #[arg(long, default_value_t = 20, help = "Min per-base Phred quality in pileup")]
    pub min_base_qual: u8,

    #[arg(long, default_value = "unstranded", help = "Library strandedness")]
    pub library_type: LibraryType,

    #[arg(long, help = "Optional: write extracted reads to FASTQ")]
    pub save_extracted: Option<PathBuf>,

    #[arg(short = 't', long, help = "Threads (default: logical CPUs)")]
    pub threads: Option<usize>,

    #[arg(long, default_value_t = 0.10, help = "Min VAF to report a novel variant (default: 0.10)")]
    pub novel_min_vaf: f64,

    #[arg(long, default_value_t = 0.30, help = "Min VAF to call a known SNP as MUT in summary (default: 0.30)")]
    pub min_mut_vaf: f64,
}
