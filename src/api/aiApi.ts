import { safeInvoke } from './tauriClient';
import type { ExtractedMetadata, RelationRecommendation, Insight } from '@/types';
import type { AiSettings, MaskedAiSettings } from '@/types';

export function aiParsePdf(projectPath: string, paperId: string) {
  return safeInvoke<ExtractedMetadata>('ai_parse_pdf', {
    projectPath,
    paperId,
  });
}

export function aiRecommendRelations(projectPath: string, newPaperIds?: string[]) {
  return safeInvoke<RelationRecommendation[]>(
    'ai_recommend_relations',
    { projectPath, newPaperIds }
  );
}

export function aiGenerateInsights(projectPath: string) {
  return safeInvoke<Insight[]>('ai_generate_insights', { projectPath });
}

export function testAiConnection(settings: AiSettings) {
  return safeInvoke<boolean>('test_ai_connection', { settings });
}

export function testAiConnectionStored() {
  return safeInvoke<boolean>('test_ai_connection_stored');
}

export function saveAiSettings(settings: AiSettings) {
  return safeInvoke<void>('save_ai_settings', { settings });
}

export function getAiSettingsMasked() {
  return safeInvoke<MaskedAiSettings>('get_ai_settings_masked');
}

export function toggleHttpServer(enabled: boolean) {
  return safeInvoke<boolean>('toggle_http_server', { enabled });
}

export function getHttpServerStatus() {
  return safeInvoke<[boolean, number]>('get_http_server_status');
}
