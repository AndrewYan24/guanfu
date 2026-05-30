export type RelationType =
  | 'supports'
  | 'opposes'
  | 'modifies'
  | 'adopts'
  | 'reinterprets';

export const relationTypes: RelationType[] = ['supports', 'opposes', 'modifies', 'adopts', 'reinterprets'];

export interface Relation {
  id: string;
  sourceId: string;
  targetId: string;
  type: RelationType;
  evidence: string;
  isManual: boolean;
  confidence?: number;
  createdAt: string;
  updatedAt: string;
}

export interface RelationRecommendation {
  sourceId: string;
  targetId: string;
  type: RelationType;
  confidence: number;
  evidence: string;
  discoveryMethod?: string;
}
