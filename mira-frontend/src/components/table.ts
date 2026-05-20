import type { SampleSummary, Concordance } from '@/types';
import { dispatch } from '@/store';

const TARGETS = ['V7_exon3', 'CE3_full', 'T878A', 'L702H', 'W742C', 'H875Y'];
const LABELS  = ['AR-V7', 'CE3', 'T878A', 'L702H', 'W742C', 'H875Y'];

type SortKey = 'sampleId' | 'cohort' | 'alterationCount' | 'therapyRecommendation';

let sortKey: SortKey = 'alterationCount';
let sortAsc = false;
let searchQuery = '';

function callBadge(call: string | undefined): string {
  if (!call) return '<span class="badge badge--warning">?</span>';
  const cls = (call === 'MUT' || call === 'PRESENCE') ? 'mut' : 'wt';
  return `<span class="badge badge--${cls}">${call}</span>`;
}

function therapyClass(rec: SampleSummary['therapyRecommendation']): string {
  const map: Record<string, string> = { Taxane: 'taxane', Darolutamide: 'darolu', 'Continue ARPI': 'arpi', Uncertain: 'uncertain' };
  return map[rec] ?? 'uncertain';
}

function concordanceBadge(c: Concordance | undefined): string {
  if (!c) return '<span style="color:var(--text-muted);font-size:11px">—</span>';
  const cfg: Record<Concordance, { label: string; color: string }> = {
    CONCORDANT:   { label: '✓ Match',      color: 'var(--green)' },
    DISCORDANT:   { label: '✗ Mismatch',   color: 'var(--red)' },
    NOT_IN_PANEL: { label: '~ Not in panel', color: 'var(--amber)' },
    UNKNOWN:      { label: '—',            color: 'var(--text-muted)' },
  };
  const { label, color } = cfg[c];
  return `<span style="font-family:var(--font-mono);font-size:11px;color:${color}">${label}</span>`;
}

function sortIndicator(key: SortKey): string {
  if (sortKey !== key) return '';
  return `<span class="sort-icon">${sortAsc ? '↑' : '↓'}</span>`;
}

export function renderTable(container: HTMLElement, samples: SampleSummary[], selectedId: string | null): void {
  const filtered = samples.filter(s =>
    s.sampleId.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const sorted = [...filtered].sort((a, b) => {
    const av = a[sortKey]; const bv = b[sortKey];
    const cmp = typeof av === 'number' ? (av - (bv as number)) : String(av).localeCompare(String(bv));
    return sortAsc ? cmp : -cmp;
  });

  const thCls = (key: SortKey) => sortKey === key ? 'sorted' : '';

  container.innerHTML = `
    <div class="table-toolbar">
      <input class="search-input" type="search" placeholder="Search sample ID…" value="${searchQuery}" aria-label="Search samples">
      <button class="btn" id="export-csv" aria-label="Export table as CSV">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>
        Export CSV
      </button>
    </div>
    <div class="summary-table-wrap">
      <table role="grid">
        <thead>
          <tr>
            <th data-key="sampleId" class="${thCls('sampleId')}" scope="col">Sample ${sortIndicator('sampleId')}</th>
            <th data-key="cohort" class="${thCls('cohort')}" scope="col">Cohort ${sortIndicator('cohort')}</th>
            ${LABELS.map(l => `<th scope="col">${l}</th>`).join('')}
            <th data-key="alterationCount" class="${thCls('alterationCount')}" scope="col"># Alt ${sortIndicator('alterationCount')}</th>
            <th data-key="therapyRecommendation" class="${thCls('therapyRecommendation')}" scope="col">Therapy ${sortIndicator('therapyRecommendation')}</th>
            <th scope="col">Known Mut.</th>
            <th scope="col">Match</th>
          </tr>
        </thead>
        <tbody>
          ${sorted.map(s => {
            const calls = TARGETS.map(t => {
              const rec = s.records.find(r => r.target.includes(t));
              return callBadge(rec?.call);
            });
            const warnIcon = s.hasLowReads
              ? '<svg width="12" height="12" style="color:var(--amber);vertical-align:middle;margin-left:4px" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-label="Low coverage warning"><path d="M12 9v4M12 17h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/></svg>'
              : '';
            return `<tr data-id="${s.sampleId}" class="${s.sampleId === selectedId ? 'selected' : ''}" tabindex="0">
              <td class="sample-id">${s.sampleId}${warnIcon}</td>
              <td class="cohort">${s.cohort}</td>
              ${calls.map(c => `<td>${c}</td>`).join('')}
              <td style="text-align:center;font-weight:600">${s.alterationCount}</td>
              <td><span class="therapy-badge therapy-badge--${therapyClass(s.therapyRecommendation)}">${s.therapyRecommendation}</span></td>
              <td style="font-family:var(--font-mono);font-size:11px;color:var(--text-muted)">${s.metadata?.arMutation ?? '—'}</td>
              <td>${concordanceBadge(s.metadata?.concordance)}</td>
            </tr>`;
          }).join('')}
        </tbody>
      </table>
    </div>
  `;

  container.querySelector('.search-input')!.addEventListener('input', e => {
    searchQuery = (e.target as HTMLInputElement).value;
    renderTable(container, samples, selectedId);
  });

  container.querySelectorAll<HTMLElement>('th[data-key]').forEach(th => {
    th.addEventListener('click', () => {
      const key = th.dataset.key as SortKey;
      if (sortKey === key) sortAsc = !sortAsc;
      else { sortKey = key; sortAsc = false; }
      renderTable(container, samples, selectedId);
    });
  });

  container.querySelectorAll<HTMLElement>('tbody tr').forEach(tr => {
    const id = tr.dataset.id!;
    tr.addEventListener('click', () => dispatch({ type: 'SELECT_SAMPLE', sampleId: id }));
    tr.addEventListener('keydown', e => {
      if ((e as KeyboardEvent).key === 'Enter') dispatch({ type: 'SELECT_SAMPLE', sampleId: id });
    });
  });

  container.querySelector('#export-csv')?.addEventListener('click', () => exportCsv(sorted));
}

function exportCsv(samples: SampleSummary[]): void {
  const header = ['Sample', 'Cohort', ...LABELS, 'Alterations', 'Therapy', 'Known_Mutation', 'MIRA_Match'];
  const rows = samples.map(s => [
    s.sampleId, s.cohort,
    ...TARGETS.map(t => s.records.find(r => r.target.includes(t))?.call ?? ''),
    String(s.alterationCount), s.therapyRecommendation,
    s.metadata?.arMutation ?? '', s.metadata?.concordance ?? '',
  ]);
  const csv = [header, ...rows].map(r => r.map(v => `"${v}"`).join(',')).join('\n');
  const blob = new Blob([csv], { type: 'text/csv' });
  const a = Object.assign(document.createElement('a'), {
    href: URL.createObjectURL(blob),
    download: 'mira-summary.csv',
  });
  a.click();
  URL.revokeObjectURL(a.href);
}
