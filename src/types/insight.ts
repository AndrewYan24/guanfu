export type InsightType =
  | 'potential-fault-line'
  | 'lack-pluralistic-testing'
  | 'method-homogeneity'
  | 'ai-insight';

export interface Insight {
  id: string;
  type: InsightType;
  title: string;
  description: string;
  relatedPaperIds: string[];
  createdAt: string;
}
