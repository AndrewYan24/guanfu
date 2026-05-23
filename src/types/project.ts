import type { Paper } from './paper';
import type { Relation } from './relation';
import type { Annotation } from './annotation';

export interface GraphPosition {
  x: number;
  y: number;
}

export interface GraphLayout {
  locked: boolean;
  zoom?: number;
  pan?: GraphPosition;
  positions: Record<string, GraphPosition>;
}

export interface ProjectSettings {
  activeAiProvider?: 'openai-compatible' | 'anthropic' | 'mock';
}

export interface Project {
  id: string;
  name: string;
  path: string;
  papers: Paper[];
  relations: Relation[];
  annotations: Annotation[];
  graphLayout: GraphLayout;
  settings?: ProjectSettings;
  createdAt: string;
  updatedAt: string;
}
