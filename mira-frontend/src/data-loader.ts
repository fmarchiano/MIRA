import { parseSummaryTsv, buildSampleSummary } from '@/parser';
import { parseMetadataCsv, normalizeSampleId, computeConcordance } from '@/metadata';
import type { SampleSummary } from '@/types';

const tsvModules = import.meta.glob('/tables/*_AR.summary.tsv', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const csvModules = import.meta.glob('/tables/metadata/*.csv', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

export function loadBundledData(): SampleSummary[] {
  // Parse metadata CSV (first one found)
  const csvRaw = Object.values(csvModules)[0] ?? '';
  const metadataMap = parseMetadataCsv(csvRaw);

  return Object.values(tsvModules).map(raw => {
    const summary = buildSampleSummary(parseSummaryTsv(raw));
    const baseName = normalizeSampleId(summary.sampleId);
    const meta = metadataMap.get(baseName);
    if (meta) {
      summary.metadata = {
        ...meta,
        concordance: computeConcordance(summary.records, meta.arMutation),
      };
    }
    return summary;
  });
}
