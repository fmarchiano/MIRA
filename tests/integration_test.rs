use std::io::Write;
use std::process::Command;

fn write_fasta(path: &str, entries: &[(&str, &str)]) {
    let mut f = std::fs::File::create(path).unwrap();
    for (name, seq) in entries {
        writeln!(f, ">{name}").unwrap();
        writeln!(f, "{seq}").unwrap();
    }
}

fn write_fastq(path: &str, reads: &[(&str, &str)]) {
    let mut f = std::fs::File::create(path).unwrap();
    for (name, seq) in reads {
        let qual = "I".repeat(seq.len()); // Phred 40
        writeln!(f, "@{name}").unwrap();
        writeln!(f, "{seq}").unwrap();
        writeln!(f, "+").unwrap();
        writeln!(f, "{qual}").unwrap();
    }
}

fn mira_bin() -> &'static str {
    env!("CARGO_BIN_EXE_mira")
}

fn summary_path(out_tsv: &std::path::Path) -> std::path::PathBuf {
    let stem = out_tsv.file_stem().unwrap().to_str().unwrap();
    out_tsv.with_file_name(format!("{stem}.summary.tsv"))
}

#[test]
fn test_snp_detection() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    // Reference: 50 bp sequence
    let ref_seq = "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTAC";
    write_fasta(ref_fa.to_str().unwrap(), &[("target1 variant_type=SNP note=_ref=ACG_alt=CCG offset=4", ref_seq)]);

    // 15 reads with SNP at position 5 (0-based 4): ref[4]='A' → 'C'
    let mut mutant = ref_seq.as_bytes().to_vec();
    mutant[4] = b'C';
    let mutant_str = String::from_utf8(mutant).unwrap();

    let mut reads: Vec<(String, String)> = Vec::new();
    for i in 0..15 {
        reads.push((format!("read{i}"), mutant_str.clone()));
    }
    for i in 15..20 {
        reads.push((format!("read{i}"), ref_seq.to_string()));
    }

    let read_refs: Vec<(&str, &str)> = reads.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect();
    write_fastq(reads_fq.to_str().unwrap(), &read_refs);

    let status = Command::new(mira_bin())
        .args([
            "-i", reads_fq.to_str().unwrap(),
            "-r", ref_fa.to_str().unwrap(),
            "-o", out_tsv.to_str().unwrap(),
            "-k", "20",
            "--min-reads", "10",
            "--no-dedup",
        ])
        .status()
        .expect("Failed to run mira");

    assert!(status.success(), "mira exited non-zero");

    let tsv = std::fs::read_to_string(&out_tsv).unwrap();
    assert!(tsv.contains("SNP"), "Expected SNP in output:\n{tsv}");
    assert!(tsv.contains("target1"), "Expected target1 in output:\n{tsv}");

    // Verify VAF: 15 mutant / 20 total ≈ 0.75
    // Find the SNP line and check position and VAF
    let snp_line = tsv.lines().find(|l| l.contains("SNP") && l.contains("target1"));
    assert!(snp_line.is_some(), "No SNP line found:\n{tsv}");
    let fields: Vec<&str> = snp_line.unwrap().split('\t').collect();
    // columns: sample target_id variant_type position ref alt supporting total frequency
    assert_eq!(fields[3], "5", "SNP should be at position 5, got: {}", fields[3]);
    let vaf: f64 = fields[8].parse().expect("frequency should be numeric");
    assert!((vaf - 0.75).abs() < 0.05, "Expected VAF ~0.75, got {vaf}");
}

#[test]
fn test_empty_fastq_no_crash() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    write_fasta(ref_fa.to_str().unwrap(), &[("target1 variant_type=SNP", "ACGTACGTACGTACGTACGTACGTACGTAC")]);
    write_fastq(reads_fq.to_str().unwrap(), &[]);

    let status = Command::new(mira_bin())
        .args([
            "-i", reads_fq.to_str().unwrap(),
            "-r", ref_fa.to_str().unwrap(),
            "-o", out_tsv.to_str().unwrap(),
            "-k", "20",
        ])
        .status()
        .expect("Failed to run mira");

    assert!(status.success());
    let tsv = std::fs::read_to_string(&out_tsv).unwrap();
    assert!(tsv.contains("PRESENCE"), "Expected PRESENCE row for zero-coverage target:\n{tsv}");
}

#[test]
fn test_no_kmer_hits_warns_not_crashes() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    write_fasta(ref_fa.to_str().unwrap(), &[("arv7_junction variant_type=SPLICE_JUNCTION", "TTTTTTTTTTTTTTTTTTTTTTTTTTTTTT")]);
    write_fastq(reads_fq.to_str().unwrap(), &[
        ("r1", "ACGCACGCACGCACGCACGCACGCACGCAC"),
        ("r2", "GCGCGCGCGCGCGCGCGCGCGCGCGCGCGC"),
    ]);

    let output = Command::new(mira_bin())
        .args([
            "-i", reads_fq.to_str().unwrap(),
            "-r", ref_fa.to_str().unwrap(),
            "-o", out_tsv.to_str().unwrap(),
            "-k", "20",
        ])
        .output()
        .expect("Failed to run mira");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("WARNING") || stderr.contains("zero"),
        "Expected warning:\n{stderr}"
    );
}

// AR-FL junction sequence (exon3_last75 + exon4_first75) used as a self-contained test reference
const JUNCTION_150: &str = "TTGATAAATTCCGAAGGAAAAATTGTCCATCTTGTCGTCTTCGGAAATGTTATGAAGCAGGGATGACTCTGGGAGCCCGGAAGCTGAAGAAACTTGGTAATCTGAAACTACAGGAGGAAGGAGAGGCTTCCAGCACCACCAGCCCCACTG";
// Simple repeating reference for SNP tests (50 bp); seq[4..7] = "ACG"
const SNP_REF_50: &str = "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTAC";

#[test]
fn test_splice_junction_presence() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa  = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    write_fasta(ref_fa.to_str().unwrap(), &[
        ("junction variant_type=SPLICE_JUNCTION", JUNCTION_150),
    ]);
    // 12 unique 50 bp reads spanning the exon3/exon4 boundary at position 75
    let reads: Vec<(String, String)> = (40usize..52)
        .map(|s| (format!("r{s}"), JUNCTION_150[s..s + 50].to_string()))
        .collect();
    write_fastq(
        reads_fq.to_str().unwrap(),
        &reads.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect::<Vec<_>>(),
    );

    let status = Command::new(mira_bin())
        .args(["-i", reads_fq.to_str().unwrap(), "-r", ref_fa.to_str().unwrap(),
               "-o", out_tsv.to_str().unwrap(), "-k", "20", "--min-reads", "10"])
        .status().unwrap();

    assert!(status.success());
    let summary = std::fs::read_to_string(summary_path(&out_tsv)).unwrap();
    assert!(summary.contains("PRESENCE"), "Expected PRESENCE:\n{summary}");
}

#[test]
fn test_low_coverage_indeterminate() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa  = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    // seq[4..7] = "ACG" — codon validation passes
    write_fasta(ref_fa.to_str().unwrap(), &[
        ("snp variant_type=SNP note=offset=4_ref=ACG_alt=CCG", SNP_REF_50),
    ]);
    // 15 unique 35 bp WT reads — below default min-coverage=30
    let reads: Vec<(String, String)> = (0usize..15)
        .map(|i| (format!("r{i}"), SNP_REF_50[i..i + 35].to_string()))
        .collect();
    write_fastq(
        reads_fq.to_str().unwrap(),
        &reads.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect::<Vec<_>>(),
    );

    let status = Command::new(mira_bin())
        .args(["-i", reads_fq.to_str().unwrap(), "-r", ref_fa.to_str().unwrap(),
               "-o", out_tsv.to_str().unwrap(), "-k", "20"])
        .status().unwrap();

    assert!(status.success());
    let summary = std::fs::read_to_string(summary_path(&out_tsv)).unwrap();
    assert!(summary.contains("INDETERMINATE"),
        "Expected INDETERMINATE with 15 reads < min_coverage=30:\n{summary}");
    assert!(!summary.contains("\tWT\t"),
        "Should not call WT below coverage threshold:\n{summary}");
}

#[test]
fn test_wt_reads_no_false_mut() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa  = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    write_fasta(ref_fa.to_str().unwrap(), &[
        ("snp variant_type=SNP note=offset=4_ref=ACG_alt=CCG", SNP_REF_50),
    ]);
    // 40 WT reads (identical — use --no-dedup) → should call WT, never MUT
    let reads: Vec<(String, String)> = (0..40)
        .map(|i| (format!("r{i}"), SNP_REF_50.to_string()))
        .collect();
    write_fastq(
        reads_fq.to_str().unwrap(),
        &reads.iter().map(|(n, s)| (n.as_str(), s.as_str())).collect::<Vec<_>>(),
    );

    let status = Command::new(mira_bin())
        .args(["-i", reads_fq.to_str().unwrap(), "-r", ref_fa.to_str().unwrap(),
               "-o", out_tsv.to_str().unwrap(), "-k", "20", "--no-dedup",
               "--min-coverage", "30"])
        .status().unwrap();

    assert!(status.success());
    let summary = std::fs::read_to_string(summary_path(&out_tsv)).unwrap();
    assert!(summary.contains("\tWT\t"), "Expected WT call:\n{summary}");
    assert!(!summary.contains("\tMUT\t"), "False positive MUT on WT reads:\n{summary}");
}

#[test]
fn test_codon_validation_fails_on_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa  = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    // seq[4..7] = "ACG" but header claims _ref=TTT → validation error → non-zero exit
    write_fasta(ref_fa.to_str().unwrap(), &[
        ("bad variant_type=SNP note=offset=4_ref=TTT_alt=CCC", SNP_REF_50),
    ]);
    write_fastq(reads_fq.to_str().unwrap(), &[("r1", SNP_REF_50)]);

    let status = Command::new(mira_bin())
        .args(["-i", reads_fq.to_str().unwrap(), "-r", ref_fa.to_str().unwrap(),
               "-o", out_tsv.to_str().unwrap(), "-k", "20"])
        .status().unwrap();

    assert!(!status.success(), "Expected non-zero exit for _ref= / sequence mismatch");
}
