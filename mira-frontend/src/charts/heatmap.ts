import Plotly from 'plotly.js-dist-min';
import type { SampleSummary } from '@/types';
import { dispatch } from '@/store';

const TARGETS = [
  { key: 'V7_exon3', label: 'AR-V7' },
  { key: 'CE3_full', label: 'CE3' },
  { key: 'T878A',    label: 'T878A' },
  { key: 'L702H',    label: 'L702H' },
  { key: 'W742C',    label: 'W742C' },
  { key: 'H875Y',    label: 'H875Y' },
];

function callToZ(call: string | undefined): number {
  if (call === 'MUT' || call === 'PRESENCE') return 1;
  if (call === 'WT'  || call === 'ABSENCE')  return 0;
  return -1;
}

export function renderHeatmap(container: HTMLElement, samples: SampleSummary[]): void {
  if (!samples.length) {
    container.innerHTML = `<div class="empty-state">
      <svg class="empty-state__icon" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2" stroke-width="1.5"/><path stroke-linecap="round" stroke-width="1.5" d="M3 9h18M3 15h18M9 3v18M15 3v18"/></svg>
      <p class="empty-state__text">Load summary TSV files to see the heatmap</p>
    </div>`;
    return;
  }

  const z = samples.map(s =>
    TARGETS.map(t => {
      const rec = s.records.find(r => r.target.includes(t.key));
      return callToZ(rec?.call);
    })
  );

  const text = samples.map(s =>
    TARGETS.map(t => {
      const rec = s.records.find(r => r.target.includes(t.key));
      if (!rec) return 'No data';
      const vafStr = rec.vaf !== null ? ` | VAF: ${(rec.vaf * 100).toFixed(1)}%` : '';
      const warn = rec.totalReads < 10 ? ' ⚠ LOW READS' : '';
      return `${s.sampleId}<br>${rec.target}<br><b>${rec.call}</b>${vafStr} | Reads: ${rec.totalReads}${warn}`;
    })
  );

  const trace = {
    type: 'heatmap' as const,
    z,
    x: TARGETS.map(t => t.label),
    y: samples.map(s => s.sampleId),
    text: text as unknown as string[],
    hovertemplate: '%{text}<extra></extra>',
    colorscale: [
      [0,   '#30363d'],
      [0.5, '#457b9d'],
      [1,   '#e63946'],
    ] as Plotly.ColorScale,
    zmin: -1, zmax: 1,
    showscale: false,
    xgap: 2,
    ygap: 2,
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor:  'transparent',
    font: { family: 'JetBrains Mono, monospace', color: '#e6edf3', size: 11 },
    margin: { l: 200, r: 20, t: 20, b: 60 },
    xaxis: { fixedrange: true, tickfont: { size: 11 } },
    yaxis: { fixedrange: true, tickfont: { size: 10, family: 'JetBrains Mono, monospace' } },
    height: Math.max(300, samples.length * 32 + 80),
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });

  (container as HTMLElement & { on: (event: string, fn: (data: Plotly.PlotMouseEvent) => void) => void })
    .on('plotly_click', (data: Plotly.PlotMouseEvent) => {
      const pt = data.points[0];
      dispatch({ type: 'SELECT_SAMPLE', sampleId: pt.y as string });
    });
}
