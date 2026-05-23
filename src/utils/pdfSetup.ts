import * as pdfjsLib from 'pdfjs-dist';

// Set worker source to the bundled worker file in public/
pdfjsLib.GlobalWorkerOptions.workerSrc = '/pdf.worker.mjs';

export { pdfjsLib };
