export type Call = 'PRESENCE' | 'ABSENCE' | 'MUT' | 'WT';
export type TargetType = 'SPLICE_JUNCTION' | 'SNP';
export type Cohort = 'WCDT' | 'TCGA' | 'UNKNOWN';
export type TherapyRec = 'Taxane' | 'Darolutamide' | 'Continue ARPI' | 'Uncertain';
export type Concordance = 'CONCORDANT' | 'DISCORDANT' | 'NOT_IN_PANEL' | 'UNKNOWN';

export interface SampleMetadata {
  sampleName: string;
  dataset: string;
  arMutation: string;
  arAmplification: string;
  concordance: Concordance;
}

export interface MiraRecord {
  sample: string;
  target: string;
  type: TargetType;
  call: Call;
  totalReads: number;
  altReads: number | null;
  vaf: number | null;
}

export interface SampleSummary {
  sampleId: string;
  cohort: Cohort;
  records: MiraRecord[];
  therapyRecommendation: TherapyRec;
  alterationCount: number;
  hasLowReads: boolean;
  metadata?: SampleMetadata;
}

export interface DashboardState {
  samples: SampleSummary[];
  selectedSample: string | null;
  filters: {
    cohort: Cohort | 'ALL';
    arv7: 'ALL' | 'PRESENCE' | 'ABSENCE';
    minReads: number;
  };
}
