import type { SampleSummary, Concordance } from '@/types';
import { dispatch } from '@/store';
import { renderVafBar } from '@/charts/vafBar';
import { renderCoverage } from '@/charts/coverage';

const CLOSE_ICON = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12"/></svg>`;
const WARN_ICON  = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 9v4M12 17h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/></svg>`;

const CHECK_ICON = `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="M20 6L9 17l-5-5"/></svg>`;
const CROSS_ICON = `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12"/></svg>`;
const DASH_ICON  = `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="M5 12h14"/></svg>`;

function therapyClass(rec: SampleSummary['therapyRecommendation']): string {
  const map: Record<string, string> = { Taxane: 'taxane', Darolutamide: 'darolu', 'Continue ARPI': 'arpi', Uncertain: 'uncertain' };
  return map[rec] ?? 'uncertain';
}

function concordanceBadge(c: Concordance): string {
  const cfg: Record<Concordance, { icon: string; label: string; cls: string }> = {
    CONCORDANT:   { icon: CHECK_ICON, label: 'Concordant',    cls: 'concordant' },
    DISCORDANT:   { icon: CROSS_ICON, label: 'Discordant',    cls: 'discordant' },
    NOT_IN_PANEL: { icon: DASH_ICON,  label: 'Not in panel',  cls: 'not_in_panel' },
    UNKNOWN:      { icon: DASH_ICON,  label: 'No metadata',   cls: 'unknown' },
  };
  const { icon, label, cls } = cfg[c];
  return `<span class="concordance-badge concordance-badge--${cls}">${icon} ${label}</span>`;
}

export function renderSampleCard(drawer: HTMLElement, sample: SampleSummary | null): void {
  const app = document.querySelector<HTMLElement>('#app')!;

  if (!sample) {
    drawer.classList.add('drawer--closed');
    app.classList.remove('drawer-open');
    return;
  }

  drawer.classList.remove('drawer--closed');
  app.classList.add('drawer-open');

  const meta = sample.metadata;
  const metaSection = meta ? `
    <div class="sample-card__section">
      <div class="sample-card__section-title">External Annotation</div>
      <div class="metadata-grid">
        <div class="metadata-item">
          <div class="metadata-item__label">Known AR Mutation</div>
          <div class="metadata-item__value">${meta.arMutation}</div>
        </div>
        <div class="metadata-item">
          <div class="metadata-item__label">AR Amplification</div>
          <div class="metadata-item__value">${meta.arAmplification}</div>
        </div>
        <div class="metadata-item">
          <div class="metadata-item__label">Dataset</div>
          <div class="metadata-item__value">${meta.dataset}</div>
        </div>
        <div class="metadata-item" style="display:flex;flex-direction:column;justify-content:center">
          <div class="metadata-item__label">MIRA Match</div>
          <div style="margin-top:4px">${concordanceBadge(meta.concordance)}</div>
        </div>
      </div>
      ${meta.concordance === 'DISCORDANT' ? `
        <div class="low-reads-warning" style="margin-top:8px">
          ${WARN_ICON} MIRA call does not match the known mutation (${meta.arMutation}).
        </div>` : ''}
      ${meta.concordance === 'NOT_IN_PANEL' ? `
        <div style="margin-top:8px;padding:8px 10px;background:color-mix(in srgb,var(--amber) 8%,transparent);border:1px solid color-mix(in srgb,var(--amber) 20%,transparent);border-radius:var(--radius);font-size:11px;color:var(--text-muted)">
          ${meta.arMutation} is outside the MIRA AR panel — no direct comparison possible.
        </div>` : ''}
    </div>` : '';

  drawer.innerHTML = `
    <button class="close-btn" aria-label="Close detail panel">${CLOSE_ICON}</button>
    <div class="sample-card">
      <div class="sample-card__header">
        <div class="sample-card__id">${sample.sampleId}</div>
        <span class="badge badge--${sample.cohort === 'WCDT' ? 'mut' : 'wt'}" style="font-size:10px">${sample.cohort}</span>
      </div>

      <div class="sample-card__section">
        <div class="sample-card__section-title">Therapy Recommendation</div>
        <span class="therapy-badge therapy-badge--${therapyClass(sample.therapyRecommendation)}">
          ${sample.therapyRecommendation}
        </span>
      </div>

      ${metaSection}

      <div class="sample-card__section">
        <div class="sample-card__section-title">MIRA AR Calls</div>
        <div class="records-list">
          ${sample.records.map(r => {
            const callCls = (r.call === 'MUT' || r.call === 'PRESENCE') ? 'mut' : 'wt';
            const vafStr = r.vaf !== null ? `VAF ${(r.vaf * 100).toFixed(1)}%` : '';
            const readsStr = `${r.totalReads} reads`;
            const warn = r.totalReads < 10 ? `<span style="color:var(--amber);margin-left:4px">${WARN_ICON}</span>` : '';
            return `<div class="record-row">
              <span class="record-row__target">${r.target.replace('AR_', '').replace('_region', '').replace('_junction', '').replace('_CE3', '')}</span>
              <span class="badge badge--${callCls}">${r.call}</span>
              <span class="record-row__meta">${[vafStr, readsStr].filter(Boolean).join(' · ')}${warn}</span>
            </div>`;
          }).join('')}
        </div>
      </div>

      ${sample.hasLowReads ? `<div class="low-reads-warning">${WARN_ICON} One or more targets have &lt;10 reads — interpret with caution.</div>` : ''}

      <div class="sample-card__section">
        <div class="sample-card__section-title">VAF (SNP targets)</div>
        <div id="vaf-chart"></div>
      </div>

      <div class="sample-card__section">
        <div class="sample-card__section-title">Read Depth</div>
        <div id="coverage-chart"></div>
      </div>
    </div>
  `;

  renderVafBar(drawer.querySelector<HTMLElement>('#vaf-chart')!, sample.records);
  renderCoverage(drawer.querySelector<HTMLElement>('#coverage-chart')!, sample.records);

  drawer.querySelector('.close-btn')!.addEventListener('click', () => {
    dispatch({ type: 'SELECT_SAMPLE', sampleId: null });
  });
}
