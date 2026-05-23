import type { ExtractedMetadata } from './paper';

export interface PdfRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Annotation {
  id: string;
  paperId: string;
  field: keyof ExtractedMetadata | 'title' | 'abstract' | 'notes';
  text: string;
  pageNumber: number;
  rects: PdfRect[];
  createdAt: string;
}
