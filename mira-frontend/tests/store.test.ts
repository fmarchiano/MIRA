import { describe, it, expect, beforeEach } from 'vitest';
import { store, dispatch, subscribe, getFiltered } from '@/store';

beforeEach(() => {
  dispatch({ type: 'RESET' });
});

describe('store dispatch', () => {
  it('starts with empty samples', () => {
    expect(store.samples).toHaveLength(0);
  });

  it('ADD_SAMPLES replaces sample list', () => {
    dispatch({ type: 'ADD_SAMPLES', samples: [{ sampleId: 'DTB-001', cohort: 'WCDT', records: [], therapyRecommendation: 'Uncertain', alterationCount: 0, hasLowReads: false }] });
    expect(store.samples).toHaveLength(1);
  });

  it('SELECT_SAMPLE sets selectedSample', () => {
    dispatch({ type: 'SELECT_SAMPLE', sampleId: 'DTB-001' });
    expect(store.selectedSample).toBe('DTB-001');
  });

  it('SET_FILTER updates cohort filter', () => {
    dispatch({ type: 'SET_FILTER', filter: { cohort: 'WCDT' } });
    expect(store.filters.cohort).toBe('WCDT');
  });
});

describe('subscribe', () => {
  it('calls subscriber on every dispatch', () => {
    let callCount = 0;
    const unsub = subscribe(() => callCount++);
    dispatch({ type: 'SELECT_SAMPLE', sampleId: null });
    expect(callCount).toBe(1);
    unsub();
    dispatch({ type: 'SELECT_SAMPLE', sampleId: null });
    expect(callCount).toBe(1);
  });
});

describe('getFiltered', () => {
  it('filters by cohort', () => {
    dispatch({
      type: 'ADD_SAMPLES', samples: [
        { sampleId: 'DTB-001', cohort: 'WCDT', records: [], therapyRecommendation: 'Uncertain', alterationCount: 0, hasLowReads: false },
        { sampleId: 'TCGA-001', cohort: 'TCGA', records: [], therapyRecommendation: 'Uncertain', alterationCount: 0, hasLowReads: false },
      ]
    });
    dispatch({ type: 'SET_FILTER', filter: { cohort: 'WCDT' } });
    expect(getFiltered()).toHaveLength(1);
    expect(getFiltered()[0].sampleId).toBe('DTB-001');
  });
});
