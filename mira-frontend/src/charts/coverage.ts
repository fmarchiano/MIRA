import Plotly from 'plotly.js-dist-min';
import type { MiraRecord } from '@/types';

export function renderCoverage(container: HTMLElement, records: MiraRecord[]): void {
  const trace = {
    type: 'bar' as const,
    orientation: 'h' as const,
    x: records.map(r => r.totalReads),
    y: records.map(r => r.target.replace('AR_', '').replace('_region', '').replace('_junction', '')),
    marker: { color: records.map(r => r.totalReads < 10 ? '#f4a261' : '#457b9d') },
    hovertemplate: '%{y}: %{x} reads<extra></extra>',
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor: 'transparent',
    font: { color: '#e6edf3', size: 10, family: 'JetBrains Mono, monospace' },
    margin: { l: 100, r: 20, t: 10, b: 30 },
    height: 160,
    xaxis: { gridcolor: '#30363d', fixedrange: true },
    yaxis: { fixedrange: true },
    shapes: [{
      type: 'line', x0: 10, x1: 10, y0: -0.5, y1: records.length - 0.5,
      line: { color: '#f4a261', dash: 'dot', width: 1 },
    }],
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });
}
