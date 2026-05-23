import type { Paper } from './paper';

export interface MetadataCandidate {
  title: string;
  authors: string[];
  year?: number;
  abstract?: string;
  doi?: string;
  source: string;
  score: number;
}

export interface MetadataResolveResult {
  candidates: MetadataCandidate[];
  applied?: boolean;
  paper?: Paper | null;
}
