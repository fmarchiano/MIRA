import { describe, it, expect } from 'vitest';
import { parseSummaryTsv, inferCohort, inferTherapy, buildSampleSummary } from '@/parser';

const SAMPLE_TSV = `sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf
DTB-034-BL-T-RNA_R1\tAR_V7_exon3_CE3_junction\tSPLICE_JUNCTION\tPRESENCE\t1414\t.\t.
DTB-034-BL-T-RNA_R1\tAR_CE3_full\tSPLICE_JUNCTION\tPRESENCE\t1230\t.\t.
DTB-034-BL-T-RNA_R1\tAR_T878A_region\tSNP\tWT\t15054\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_L702H_region\tSNP\tWT\t3814\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_W742C_region\tSNP\tWT\t5080\t0\t0.0000
DTB-034-BL-T-RNA_R1\tAR_H875Y_region\tSNP\tWT\t14720\t0\t0.0000`;

describe('parseSummaryTsv', () => {
  it('parses all 6 rows', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records).toHaveLength(6);
  });

  it('sets vaf null for splice targets', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records[0].vaf).toBeNull();
    expect(records[0].altReads).toBeNull();
  });

  it('parses SNP vaf as float', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records[2].vaf).toBe(0.0);
    expect(records[2].altReads).toBe(0);
  });

  it('parses totalReads as integer', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(records[0].totalReads).toBe(1414);
  });
});

describe('inferCohort', () => {
  it('identifies WCDT by DTB- prefix', () => expect(inferCohort('DTB-034-BL')).toBe('WCDT'));
  it('identifies TCGA by TCGA- prefix', () => expect(inferCohort('TCGA-EJ-7314')).toBe('TCGA'));
  it('returns UNKNOWN for unrecognized prefixes', () => expect(inferCohort('SAMPLE-001')).toBe('UNKNOWN'));
});

describe('inferTherapy', () => {
  it('returns Taxane when AR-V7 is PRESENCE', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    expect(inferTherapy(records)).toBe('Taxane');
  });

  it('returns Darolutamide when T878A is MUT', () => {
    const tsv = SAMPLE_TSV
      .replace('PRESENCE\t1414', 'ABSENCE\t1414')
      .replace('PRESENCE\t1230', 'ABSENCE\t1230')
      .replace('WT\t15054\t0\t0.0000', 'MUT\t15054\t150\t0.0100');
    expect(inferTherapy(parseSummaryTsv(tsv))).toBe('Darolutamide');
  });

  it('returns Continue ARPI when all WT/ABSENCE', () => {
    const tsv = SAMPLE_TSV
      .replace('PRESENCE\t1414', 'ABSENCE\t1414')
      .replace('PRESENCE\t1230', 'ABSENCE\t1230');
    expect(inferTherapy(parseSummaryTsv(tsv))).toBe('Continue ARPI');
  });
});

describe('buildSampleSummary', () => {
  it('flags hasLowReads when any target has < 10 reads', () => {
    const lowTsv = SAMPLE_TSV.replace('WT\t3814', 'WT\t5');
    const summary = buildSampleSummary(parseSummaryTsv(lowTsv));
    expect(summary.hasLowReads).toBe(true);
  });

  it('counts alterationCount as MUT + PRESENCE calls', () => {
    const records = parseSummaryTsv(SAMPLE_TSV);
    const summary = buildSampleSummary(records);
    expect(summary.alterationCount).toBe(2); // 2 PRESENCE (V7 + CE3)
  });
});
