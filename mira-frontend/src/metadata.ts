import type { SampleMetadata, MiraRecord, Concordance } from './types';

// Maps protein notation from metadata to MIRA target key substrings
const MUTATION_TO_TARGET: Record<string, string> = {
  'p.T878A': 'T878A',
  'p.L702H': 'L702H',
  'p.W742C': 'W742C',
  'p.H875Y': 'H875Y',
};

// Strip RNA-seq suffixes to get the base sample name for lookup
// DTB-187-BL-T-RNA_R1 → DTB-187-BL
// TCGA-FC-A8O0-01A-41R-A37L-07 → TCGA-FC-A8O0-01A
export function normalizeSampleId(sampleId: string): string {
  if (sampleId.startsWith('DTB-')) {
    return sampleId.replace(/-T-RNA.*$/i, '');
  }
  if (sampleId.startsWith('TCGA-')) {
    // Keep first 4 hyphen-separated parts: TCGA-XX-XXXX-01A
    const parts = sampleId.split('-');
    return parts.slice(0, 4).join('-');
  }
  return sampleId;
}

export function computeConcordance(records: MiraRecord[], arMutation: string): Concordance {
  if (arMutation === 'WT') {
    const snpCalls = records.filter(r => r.type === 'SNP');
    if (!snpCalls.length) return 'UNKNOWN';
    return snpCalls.every(r => r.call === 'WT') ? 'CONCORDANT' : 'DISCORDANT';
  }

  const targetKey = MUTATION_TO_TARGET[arMutation];
  if (!targetKey) return 'NOT_IN_PANEL'; // e.g. p.A597T, p.Q58L, p.R608Q

  const rec = records.find(r => r.target.includes(targetKey));
  if (!rec) return 'UNKNOWN';
  return rec.call === 'MUT' ? 'CONCORDANT' : 'DISCORDANT';
}

export function parseMetadataCsv(raw: string): Map<string, SampleMetadata> {
  const lines = raw.trim().split('\n');
  const map = new Map<string, SampleMetadata>();
  for (const line of lines.slice(1)) {
    const [sampleName, dataset, arMutation, arAmplification] = line.split(',');
    if (!sampleName) continue;
    map.set(sampleName.trim(), {
      sampleName: sampleName.trim(),
      dataset: dataset?.trim() ?? '',
      arMutation: arMutation?.trim() ?? '',
      arAmplification: arAmplification?.trim() ?? '',
      concordance: 'UNKNOWN',
    });
  }
  return map;
}
