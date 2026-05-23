import { safeInvoke } from './tauriClient';
import type { Relation, GraphLayout, Insight } from '@/types';

export function addRelation(projectPath: string, relation: Relation) {
  return safeInvoke<Relation>('add_relation', { projectPath, relation });
}

export function updateRelation(projectPath: string, relation: Relation) {
  return safeInvoke<Relation>('update_relation', { projectPath, relation });
}

export function deleteRelation(projectPath: string, relationId: string) {
  return safeInvoke<void>('delete_relation', { projectPath, relationId });
}

export function saveGraphLayout(projectPath: string, layout: GraphLayout) {
  return safeInvoke<void>('save_graph_layout', { projectPath, layout });
}

export function runInsightAnalysis(projectPath: string) {
  return safeInvoke<Insight[]>('run_insight_analysis', { projectPath });
}

export function loadSavedInsights(projectPath: string) {
  return safeInvoke<Insight[]>('load_saved_insights', { projectPath });
}

export function saveInsights(projectPath: string, insights: Insight[]) {
  return safeInvoke<void>('save_insights', { projectPath, insights });
}
