import { parseSummaryTsv, buildSampleSummary } from '@/parser';
import { dispatch } from '@/store';

const UPLOAD_ICON = `<svg class="upload-zone__icon" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
  <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"/>
</svg>`;

export function mountUpload(container: HTMLElement): void {
  container.innerHTML = `
    <div class="upload-zone" id="drop-zone" role="button" tabindex="0" aria-label="Upload summary TSV files">
      ${UPLOAD_ICON}
      <label class="upload-zone__label">
        <strong>Choose files</strong> or drag &amp; drop<br>
        <span style="font-size:11px">*_AR.summary.tsv</span>
      </label>
      <input type="file" id="file-input" accept=".tsv" multiple />
    </div>
    <div class="upload-count" id="upload-count" aria-live="polite"></div>
  `;

  const zone = container.querySelector<HTMLDivElement>('#drop-zone')!;
  const input = container.querySelector<HTMLInputElement>('#file-input')!;
  const count = container.querySelector<HTMLDivElement>('#upload-count')!;

  zone.addEventListener('click', () => input.click());
  zone.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') input.click(); });
  zone.addEventListener('dragover', e => { e.preventDefault(); zone.classList.add('drag-over'); });
  zone.addEventListener('dragleave', () => zone.classList.remove('drag-over'));
  zone.addEventListener('drop', e => {
    e.preventDefault();
    zone.classList.remove('drag-over');
    if (e.dataTransfer?.files) processFiles(Array.from(e.dataTransfer.files), count);
  });
  input.addEventListener('change', () => {
    if (input.files) processFiles(Array.from(input.files), count);
  });
}

async function processFiles(files: File[], countEl: HTMLElement): Promise<void> {
  const summaryFiles = files.filter(f => f.name.endsWith('.summary.tsv'));
  if (!summaryFiles.length) return;

  const summaries = await Promise.all(
    summaryFiles.map(async f => {
      const raw = await f.text();
      return buildSampleSummary(parseSummaryTsv(raw));
    })
  );

  dispatch({ type: 'ADD_SAMPLES', samples: summaries });
  countEl.textContent = `${summaries.length} sample${summaries.length !== 1 ? 's' : ''} loaded`;
}
