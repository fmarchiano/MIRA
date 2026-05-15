mod aligner;
mod caller;
mod cli;
mod index;
mod output;
mod scanner;
mod types;

use anyhow::Result;
use clap::Parser;
use std::path::Path;

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    if let Some(t) = args.threads {
        rayon::ThreadPoolBuilder::new().num_threads(t).build_global()?;
    }

    // Build k-mer index from reference FASTA
    eprintln!("[mira] Building k-mer index from {}", args.reference.display());
    let kmer_index = index::KmerIndex::build(&args.reference, args.kmer_size)?;
    eprintln!("[mira] {} targets loaded, k={}", kmer_index.targets.len(), args.kmer_size);

    // Scan FASTQ
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

    // Optionally save extracted reads
    if let Some(ref save_path) = args.save_extracted {
        save_extracted_reads(&scan.hits, save_path)?;
        eprintln!("[mira] Extracted reads written to {}", save_path.display());
    }

    // Build pileups via Smith-Waterman alignment
    eprintln!("[mira] Aligning and building pileups...");
    let pileups = aligner::build_pileups(&scan.hits, &kmer_index.targets, args.min_base_qual)?;

    // Call variants
    let variants = caller::call_variants(&pileups, &kmer_index.targets, args.min_reads);
    eprintln!("[mira] {} variant/presence rows called", variants.len());

    // Write TSV
    let sample = r1.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sample");
    // strip .fastq if the stem is still e.g. "sample.fastq" after removing .gz
    let sample = sample.trim_end_matches(".fastq").trim_end_matches(".fq");

    output::write_tsv(&args.output, sample, &variants, &kmer_index.targets)?;
    eprintln!("[mira] Results written to {}", args.output.display());

    let stem = args.output.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();

    let summary_path = args.output.with_file_name(format!("{}.summary.tsv", stem));
    output::write_summary(&summary_path, sample, &variants, &kmer_index.targets, args.min_reads, args.min_mut_vaf)?;
    eprintln!("[mira] Summary written to {}", summary_path.display());

    let novel_path = args.output.with_file_name(format!("{}.novel.tsv", stem));
    output::write_novel(&novel_path, sample, &variants, &kmer_index.targets, args.novel_min_vaf)?;
    eprintln!("[mira] Novel variants written to {}", novel_path.display());

    Ok(())
}

fn save_extracted_reads(hits: &[types::Hit], path: &Path) -> Result<()> {
    use std::io::Write;
    let file = std::fs::File::create(path)?;
    let mut w = std::io::BufWriter::new(file);
    let mut seen: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
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
