import '@/style.css';
import { mountUpload } from '@/components/upload';
import { mountFilters } from '@/components/filters';
import { renderHeatmap } from '@/charts/heatmap';
import { renderTable } from '@/components/table';
import { renderSampleCard } from '@/components/sampleCard';
import { subscribe, getFiltered, store, dispatch } from '@/store';
import { loadBundledData } from '@/data-loader';

mountUpload(document.querySelector<HTMLElement>('#upload-zone')!);
mountFilters(document.querySelector<HTMLElement>('#filter-panel')!);

const heatmapEl = document.querySelector<HTMLElement>('#heatmap-chart')!;
const tableEl   = document.querySelector<HTMLElement>('#summary-table')!;
const drawerEl  = document.querySelector<HTMLElement>('#detail-drawer')!;

renderHeatmap(heatmapEl, []);
renderTable(tableEl, [], null);
renderSampleCard(drawerEl, null);

subscribe(() => {
  const samples = getFiltered();
  const selected = store.selectedSample
    ? store.samples.find(s => s.sampleId === store.selectedSample) ?? null
    : null;
  renderHeatmap(heatmapEl, samples);
  renderTable(tableEl, samples, store.selectedSample);
  renderSampleCard(drawerEl, selected);
});

const bundled = loadBundledData();
if (bundled.length) dispatch({ type: 'ADD_SAMPLES', samples: bundled });
