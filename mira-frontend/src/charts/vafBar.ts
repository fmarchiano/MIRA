import Plotly from 'plotly.js-dist-min';
import type { MiraRecord } from '@/types';

export function renderVafBar(container: HTMLElement, records: MiraRecord[]): void {
  const snp = records.filter(r => r.type === 'SNP');
  if (!snp.length) {
    container.innerHTML = '<p style="color:var(--text-muted);font-size:12px;padding:8px 0">No SNP targets</p>';
    return;
  }

  const trace = {
    type: 'bar' as const,
    orientation: 'h' as const,
    x: snp.map(r => r.vaf ?? 0),
    y: snp.map(r => r.target.replace('AR_', '').replace('_region', '')),
    marker: { color: snp.map(r => (r.vaf ?? 0) >= 0.3 ? '#e63946' : '#457b9d') },
    hovertemplate: '%{y}: %{x:.1%}<extra></extra>',
  };

  const layout: Partial<Plotly.Layout> = {
    paper_bgcolor: 'transparent',
    plot_bgcolor: 'transparent',
    font: { color: '#e6edf3', size: 10, family: 'JetBrains Mono, monospace' },
    margin: { l: 100, r: 20, t: 10, b: 30 },
    height: 140,
    xaxis: { range: [0, 1], tickformat: '.0%', gridcolor: '#30363d', fixedrange: true },
    yaxis: { fixedrange: true },
    shapes: [{
      type: 'line', x0: 0.3, x1: 0.3, y0: -0.5, y1: snp.length - 0.5,
      line: { color: '#f4a261', dash: 'dot', width: 1 },
    }],
  };

  Plotly.react(container, [trace], layout, { responsive: true, displayModeBar: false });
}
