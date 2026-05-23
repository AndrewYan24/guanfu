import { safeInvoke } from './tauriClient';
import type { MetadataCandidate, MetadataResolveResult, Paper } from '@/types';

export function resolveMetadata(projectPath: string, paperId: string) {
  return safeInvoke<MetadataResolveResult>('resolve_metadata', {
    projectPath,
    paperId,
  });
}

export function searchMetadataCandidates(
  projectPath: string,
  paperId: string
) {
  return safeInvoke<MetadataCandidate[]>('search_metadata_candidates', {
    projectPath,
    paperId,
  });
}

export function applyMetadataCandidate(
  projectPath: string,
  paperId: string,
  candidate: MetadataCandidate
) {
  return safeInvoke<Paper>('apply_metadata_candidate', {
    projectPath,
    paperId,
    candidate,
  });
}
