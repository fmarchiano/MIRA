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

fn mira_bin() -> String {
    // works from `cargo test` working dir
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary
    if path.ends_with("deps") { path.pop(); }
    path.push("mira");
    path.to_string_lossy().to_string()
}

#[test]
fn test_snp_detection() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    // Reference: 50 bp sequence
    let ref_seq = "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTAC";
    write_fasta(ref_fa.to_str().unwrap(), &[("target1", ref_seq)]);

    // 15 reads with T→A SNP at position 5 (0-based 4)
    // ref[4] = 'A', we mutate to 'C'
    let mut mutant = ref_seq.as_bytes().to_vec();
    mutant[4] = b'C';
    let mutant_str = String::from_utf8(mutant).unwrap();

    let mut reads: Vec<(String, String)> = Vec::new();
    for i in 0..15 {
        reads.push((format!("read{i}"), mutant_str.clone()));
    }
    // 5 wild-type reads
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
        ])
        .status()
        .expect("Failed to run mira");

    assert!(status.success(), "mira exited non-zero");

    let tsv = std::fs::read_to_string(&out_tsv).unwrap();
    assert!(tsv.contains("SNP"), "Expected SNP in output:\n{tsv}");
    assert!(tsv.contains("target1"), "Expected target1 in output:\n{tsv}");
}

#[test]
fn test_empty_fastq_no_crash() {
    let dir = tempfile::tempdir().unwrap();
    let ref_fa = dir.path().join("ref.fa");
    let reads_fq = dir.path().join("reads.fq");
    let out_tsv = dir.path().join("out.tsv");

    write_fasta(ref_fa.to_str().unwrap(), &[("target1", "ACGTACGTACGTACGTACGTACGTACGTAC")]);
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

    write_fasta(ref_fa.to_str().unwrap(), &[("arv7_junction", "TTTTTTTTTTTTTTTTTTTTTTTTTTTTTT")]);
    // reads completely unrelated
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
    assert!(stderr.contains("WARNING") || stderr.contains("zero"), "Expected warning:\n{stderr}");
}
