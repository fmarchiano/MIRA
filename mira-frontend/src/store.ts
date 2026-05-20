import type { DashboardState, SampleSummary, Cohort } from './types';

type Action =
  | { type: 'ADD_SAMPLES'; samples: SampleSummary[] }
  | { type: 'SELECT_SAMPLE'; sampleId: string | null }
  | { type: 'SET_FILTER'; filter: Partial<DashboardState['filters']> }
  | { type: 'RESET' };

export const store: DashboardState = {
  samples: [],
  selectedSample: null,
  filters: { cohort: 'ALL', arv7: 'ALL', minReads: 0 },
};

const subscribers = new Set<() => void>();

export function subscribe(fn: () => void): () => void {
  subscribers.add(fn);
  return () => subscribers.delete(fn);
}

export function dispatch(action: Action): void {
  switch (action.type) {
    case 'ADD_SAMPLES':
      store.samples = action.samples;
      break;
    case 'SELECT_SAMPLE':
      store.selectedSample = action.sampleId;
      break;
    case 'SET_FILTER':
      Object.assign(store.filters, action.filter);
      break;
    case 'RESET':
      store.samples = [];
      store.selectedSample = null;
      store.filters = { cohort: 'ALL', arv7: 'ALL', minReads: 0 };
      break;
  }
  subscribers.forEach(fn => fn());
}

export function getFiltered(): SampleSummary[] {
  const { cohort, arv7, minReads } = store.filters;
  return store.samples.filter(s => {
    if (cohort !== 'ALL' && s.cohort !== (cohort as Cohort)) return false;
    if (arv7 !== 'ALL') {
      const v7 = s.records.find(r => r.target.includes('V7_exon3'));
      if (v7?.call !== arv7) return false;
    }
    if (minReads > 0 && s.records.some(r => r.totalReads < minReads)) return false;
    return true;
  });
}
