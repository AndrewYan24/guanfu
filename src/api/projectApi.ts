import { safeInvoke } from './tauriClient';
import type { Project } from '@/types';

export function createProject(name: string, baseDir: string) {
  return safeInvoke<Project>('create_project', { name, baseDir });
}

export function openProject(projectPath: string) {
  return safeInvoke<Project>('open_project', { projectPath });
}

export function saveProject(project: Project) {
  return safeInvoke<void>('save_project', { project });
}

export function getRecentProject() {
  return safeInvoke<string | null>('get_recent_project');
}

export function setRecentProject(projectPath: string) {
  return safeInvoke<void>('set_recent_project', { projectPath });
}

export function getDefaultProjectDir() {
  return safeInvoke<string>('get_default_project_dir');
}
