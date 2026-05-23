export type RelationType =
  | 'supports'
  | 'opposes'
  | 'modifies'
  | 'adopts'
  | 'reinterprets';

export const relationLabels: Record<RelationType, string> = {
  supports: '支持',
  opposes: '反对',
  modifies: '修正 / 限定',
  adopts: '继承 / 采用',
  reinterprets: '再诠释',
};

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
}
