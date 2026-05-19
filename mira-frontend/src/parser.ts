import type { MiraRecord, TargetType, Call, Cohort, TherapyRec, SampleSummary } from './types';

export function parseSummaryTsv(raw: string): MiraRecord[] {
  const lines = raw.trim().split('\n');
  return lines.slice(1).map(line => {
    const cols = line.split('\t');
    return {
      sample: cols[0],
      target: cols[1],
      type: cols[2] as TargetType,
      call: cols[3] as Call,
      totalReads: parseInt(cols[4]) || 0,
      altReads: cols[5] === '.' ? null : parseInt(cols[5]),
      vaf: cols[6] === '.' ? null : parseFloat(cols[6]),
    };
  });
}

export function inferCohort(sampleId: string): Cohort {
  if (sampleId.startsWith('DTB-')) return 'WCDT';
  if (sampleId.startsWith('TCGA-')) return 'TCGA';
  return 'UNKNOWN';
}

export function inferTherapy(records: MiraRecord[]): TherapyRec {
  const get = (target: string) => records.find(r => r.target.includes(target))?.call;
  if (get('V7_exon3') === 'PRESENCE') return 'Taxane';
  if (get('T878A') === 'MUT' || get('L702H') === 'MUT') return 'Darolutamide';
  if (records.every(r => r.call === 'WT' || r.call === 'ABSENCE')) return 'Continue ARPI';
  return 'Uncertain';
}

export function buildSampleSummary(records: MiraRecord[]): SampleSummary {
  const sampleId = records[0]?.sample ?? 'unknown';
  return {
    sampleId,
    cohort: inferCohort(sampleId),
    records,
    therapyRecommendation: inferTherapy(records),
    alterationCount: records.filter(r => r.call === 'MUT' || r.call === 'PRESENCE').length,
    hasLowReads: records.some(r => r.totalReads < 10),
  };
}
