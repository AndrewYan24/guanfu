import { safeInvoke } from './tauriClient';
import type { Paper } from '@/types';

export function importPdfs(projectPath: string, filePaths: string[]) {
  return safeInvoke<Paper[]>('import_pdfs', { projectPath, filePaths });
}

export function updatePaper(projectPath: string, paper: Paper) {
  return safeInvoke<Paper>('update_paper', { projectPath, paper });
}

export function deletePaper(projectPath: string, paperId: string) {
  return safeInvoke<void>('delete_paper', { projectPath, paperId });
}

export function extractPdfText(projectPath: string, paperId: string) {
  return safeInvoke<string>('extract_pdf_text', { projectPath, paperId });
}

export function getPdfFileUrl(projectPath: string, paperId: string) {
  return safeInvoke<string>('get_pdf_file_url', { projectPath, paperId });
}

export function readPdfFile(projectPath: string, paperId: string) {
  return safeInvoke<string>('read_pdf_file', { projectPath, paperId });
}
