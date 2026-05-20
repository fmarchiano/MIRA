import { describe, it, expect } from 'vitest';
import { normalizeSampleId, computeConcordance, parseMetadataCsv } from '@/metadata';
import { parseSummaryTsv } from '@/parser';

describe('normalizeSampleId', () => {
  it('strips WCDT RNA-seq suffix', () => {
    expect(normalizeSampleId('DTB-187-BL-T-RNA_R1')).toBe('DTB-187-BL');
  });
  it('strips TCGA barcode suffix', () => {
    expect(normalizeSampleId('TCGA-EJ-7314-01A-31R-2118-07')).toBe('TCGA-EJ-7314-01A');
  });
  it('passes through unknown formats unchanged', () => {
    expect(normalizeSampleId('SAMPLE-001')).toBe('SAMPLE-001');
  });
});

describe('computeConcordance', () => {
  const TSV_T878A_MUT = `sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf
DTB-187-BL-T-RNA_R1\tAR_V7_exon3_CE3_junction\tSPLICE_JUNCTION\tABSENCE\t800\t.\t.
DTB-187-BL-T-RNA_R1\tAR_CE3_full\tSPLICE_JUNCTION\tABSENCE\t700\t.\t.
DTB-187-BL-T-RNA_R1\tAR_T878A_region\tSNP\tMUT\t2000\t440\t0.2200
DTB-187-BL-T-RNA_R1\tAR_L702H_region\tSNP\tWT\t1800\t0\t0.0000
DTB-187-BL-T-RNA_R1\tAR_W742C_region\tSNP\tWT\t1600\t0\t0.0000
DTB-187-BL-T-RNA_R1\tAR_H875Y_region\tSNP\tWT\t1900\t0\t0.0000`;

  const TSV_ALL_WT = `sample\ttarget\ttype\tcall\ttotal_reads\talt_reads\tvaf
DTB-097-T-RNA_R1\tAR_V7_exon3_CE3_junction\tSPLICE_JUNCTION\tABSENCE\t500\t.\t.
DTB-097-T-RNA_R1\tAR_CE3_full\tSPLICE_JUNCTION\tABSENCE\t400\t.\t.
DTB-097-T-RNA_R1\tAR_T878A_region\tSNP\tWT\t1000\t0\t0.0000
DTB-097-T-RNA_R1\tAR_L702H_region\tSNP\tWT\t900\t0\t0.0000
DTB-097-T-RNA_R1\tAR_W742C_region\tSNP\tWT\t800\t0\t0.0000
DTB-097-T-RNA_R1\tAR_H875Y_region\tSNP\tWT\t1100\t0\t0.0000`;

  it('CONCORDANT when metadata=p.T878A and MIRA=MUT', () => {
    expect(computeConcordance(parseSummaryTsv(TSV_T878A_MUT), 'p.T878A')).toBe('CONCORDANT');
  });

  it('DISCORDANT when metadata=p.L702H but MIRA=WT', () => {
    expect(computeConcordance(parseSummaryTsv(TSV_T878A_MUT), 'p.L702H')).toBe('DISCORDANT');
  });

  it('CONCORDANT when metadata=WT and all MIRA SNPs are WT', () => {
    expect(computeConcordance(parseSummaryTsv(TSV_ALL_WT), 'WT')).toBe('CONCORDANT');
  });

  it('DISCORDANT when metadata=WT but MIRA detects a MUT', () => {
    expect(computeConcordance(parseSummaryTsv(TSV_T878A_MUT), 'WT')).toBe('DISCORDANT');
  });

  it('NOT_IN_PANEL for mutations outside MIRA targets', () => {
    expect(computeConcordance(parseSummaryTsv(TSV_ALL_WT), 'p.A597T')).toBe('NOT_IN_PANEL');
    expect(computeConcordance(parseSummaryTsv(TSV_ALL_WT), 'p.Q58L')).toBe('NOT_IN_PANEL');
    expect(computeConcordance(parseSummaryTsv(TSV_ALL_WT), 'p.R608Q')).toBe('NOT_IN_PANEL');
  });
});

describe('parseMetadataCsv', () => {
  const CSV = `Sample_Name,Dataset,AR_Mutation,AR_Amplification,GDC_Download_UUID,Sample_UUID
DTB-187-BL,WCDT-MCRPC,p.T878A,Unknown,uuid1,uuid2
TCGA-HC-A8CY-01A,TCGA-PRAD,p.Q58L,Neutral/Loss,uuid3,uuid4`;

  it('parses all rows into a map keyed by sample name', () => {
    const map = parseMetadataCsv(CSV);
    expect(map.size).toBe(2);
    expect(map.get('DTB-187-BL')?.arMutation).toBe('p.T878A');
    expect(map.get('TCGA-HC-A8CY-01A')?.arMutation).toBe('p.Q58L');
  });

  it('sets concordance to UNKNOWN on parse (computed later)', () => {
    const map = parseMetadataCsv(CSV);
    expect(map.get('DTB-187-BL')?.concordance).toBe('UNKNOWN');
  });
});
