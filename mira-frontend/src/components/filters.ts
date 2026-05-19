import { dispatch } from '@/store';

export function mountFilters(container: HTMLElement): void {
  container.innerHTML = `
    <div style="margin-bottom:4px;font-size:11px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;color:var(--text-muted)">Filters</div>

    <div class="filter-group">
      <label class="filter-label" for="filter-cohort">Cohort</label>
      <select class="filter-select" id="filter-cohort" aria-label="Filter by cohort">
        <option value="ALL">All cohorts</option>
        <option value="WCDT">WCDT (mCRPC)</option>
        <option value="TCGA">TCGA-PRAD</option>
      </select>
    </div>

    <div class="filter-group">
      <label class="filter-label" for="filter-arv7">AR-V7 Status</label>
      <select class="filter-select" id="filter-arv7" aria-label="Filter by AR-V7 status">
        <option value="ALL">All</option>
        <option value="PRESENCE">AR-V7+</option>
        <option value="ABSENCE">AR-V7−</option>
      </select>
    </div>

    <div class="filter-group">
      <label class="filter-label" for="filter-reads">Min Reads</label>
      <input class="filter-input" type="number" id="filter-reads" min="0" value="0" aria-label="Minimum reads threshold">
    </div>
  `;

  container.querySelector<HTMLSelectElement>('#filter-cohort')!.addEventListener('change', e => {
    dispatch({ type: 'SET_FILTER', filter: { cohort: (e.target as HTMLSelectElement).value as 'ALL' | 'WCDT' | 'TCGA' } });
  });

  container.querySelector<HTMLSelectElement>('#filter-arv7')!.addEventListener('change', e => {
    dispatch({ type: 'SET_FILTER', filter: { arv7: (e.target as HTMLSelectElement).value as 'ALL' | 'PRESENCE' | 'ABSENCE' } });
  });

  container.querySelector<HTMLInputElement>('#filter-reads')!.addEventListener('input', e => {
    dispatch({ type: 'SET_FILTER', filter: { minReads: parseInt((e.target as HTMLInputElement).value) || 0 } });
  });
}
