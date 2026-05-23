export type MetadataSource =
  | 'manual'
  | 'ai'
  | 'crossref'
  | 'cnki'
  | 'arxiv'
  | 'openalex'
  | 'xmp'
  | 'filename'
  | 'mock';

export interface ExtractedMetadata {
  title?: string;
  authors?: string[];
  year?: number;
  abstract?: string;
  researchQuestion: string;
  coreClaim: string;
  assumptions: string;
  theoreticalPerspective: string;
  methodology: string;
  findings: string;
  limitations: string;
  selfPositioning: string;
  version: number;
  lastUpdated: string;
  source: MetadataSource;
  isAiGenerated?: boolean;
}

export interface Paper {
  id: string;
  title: string;
  authors: string[];
  year?: number;
  abstract?: string;
  filePath: string;
  metadata?: ExtractedMetadata;
  tags: string[];
  notes: string;
  createdAt: string;
  updatedAt: string;
}
