import type { Paper, Relation } from '@/types';
import type { ElementDefinition } from 'cytoscape';

export function papersToElements(
  papers: Paper[],
  relations: Relation[],
  labels?: Record<string, string>
): ElementDefinition[] {
  const elements: ElementDefinition[] = [];

  // Build set of valid paper IDs for edge filtering
  const paperIds = new Set(papers.map((p) => p.id));

  // Nodes from papers
  for (const paper of papers) {
    const firstAuthor = paper.authors[0]?.split(' ').pop() ?? '?';
    const label = paper.year
      ? `${firstAuthor} ${paper.year}`
      : firstAuthor;

    // Count incoming opposing relations
    const opposingCount = relations.filter(
      (r) => r.targetId === paper.id && (r.type === 'opposes' || r.type === 'modifies')
    ).length;

    const relationCount = relations.filter(
      (r) => r.sourceId === paper.id || r.targetId === paper.id
    ).length;

    // Controversial nodes get a size boost
    const controversyBonus = opposingCount >= 2 ? opposingCount * 4 : 0;

    // Estimate label width to ensure node is large enough
    // ~6.5px per char at 11px font (conservative), plus padding
    const labelWidth = label.length * 6.5 + 16;
    const minSize = Math.max(50, Math.ceil(labelWidth / 10) * 10);

    elements.push({
      data: {
        id: paper.id,
        label,
        title: paper.title,
        authors: paper.authors,
        year: paper.year,
        coreClaim: paper.metadata?.coreClaim ?? '',
        isControversial: opposingCount >= 2,
      },
      classes: [
        opposingCount >= 2 ? 'controversial' : '',
        paper.metadata?.isAiGenerated ? 'ai-parsed' : '',
      ]
        .filter(Boolean)
        .join(' '),
      // Size based on relation count and label length, with bonus for controversial nodes
      style: {
        width: Math.max(minSize, 50 + relationCount * 4 + controversyBonus),
        height: Math.max(minSize, 50 + relationCount * 4 + controversyBonus),
      },
    });
  }

  // Edges from relations (only if both source and target papers exist)
  for (const relation of relations) {
    if (!paperIds.has(relation.sourceId) || !paperIds.has(relation.targetId)) continue;
    elements.push({
      data: {
        id: relation.id,
        source: relation.sourceId,
        target: relation.targetId,
        type: relation.type,
        evidence: relation.evidence,
        label: relationTypeToLabel(relation.type, labels),
      },
      classes: `relation-${relation.type}`,
    });
  }

  return elements;
}

function relationTypeToLabel(type: string, labels?: Record<string, string>): string {
  return labels?.[type] ?? type;
}

function cssVar(name: string, fallback: string): string {
  const val = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return val || fallback;
}

export function getCytoscapeStyles(): cytoscape.StylesheetCSS[] {
  const nodeBg = cssVar('--cy-node-bg', '#EAEAEA');
  const nodeBorder = cssVar('--cy-node-border', '#333333');
  const nodeText = cssVar('--cy-node-text', '#1A1A1A');
  const nodeSelected = cssVar('--cy-node-selected', '#000000');
  const nodeControversial = cssVar('--cy-node-controversial', '#000000');
  const edgeText = cssVar('--cy-edge-text', '#888888');
  const edgeTextBg = cssVar('--cy-edge-text-bg', '#FFFFFF');
  const supports = cssVar('--cy-supports', '#2C2C2C');
  const opposes = cssVar('--cy-opposes', '#1A1A1A');
  const modifies = cssVar('--cy-modifies', '#4D4D4D');
  const adopts = cssVar('--cy-adopts', '#808080');
  const reinterprets = cssVar('--cy-reinterprets', '#B3B3B3');

  return [
    {
      selector: 'node',
      css: {
        'background-color': nodeBg,
        'border-width': 2,
        'border-color': nodeBorder,
        label: 'data(label)',
        'font-size': '11px',
        'font-family': '"HarmonyOS Sans", -apple-system, sans-serif',
        color: nodeText,
        'text-valign': 'center',
        'text-halign': 'center',
        'text-wrap': 'wrap',
        'text-max-width': '40px',
        'overlay-opacity': 0,
      },
    },
    {
      selector: 'node:selected',
      css: {
        'border-color': nodeSelected,
        'border-width': 3,
      },
    },
    {
      selector: 'node.controversial',
      css: {
        'border-style': 'dashed',
        'border-color': nodeControversial,
      },
    },
    {
      selector: 'node:active',
      css: {
        'overlay-opacity': 0,
      },
    },
    {
      selector: 'edge',
      css: {
        width: 1.5,
        'curve-style': 'bezier',
        'control-point-step-size': 50,
        'target-arrow-shape': 'triangle',
        'arrow-scale': 0.8,
        'overlay-opacity': 0,
        label: 'data(label)',
        'font-size': '10px',
        'font-family': '"HarmonyOS Sans", -apple-system, sans-serif',
        color: edgeText,
        'text-rotation': 'autorotate',
        'text-margin-y': -8,
        'text-wrap': 'wrap',
        'text-max-width': '60px',
        'text-background-color': edgeTextBg,
        'text-background-opacity': 0.85,
        'text-background-padding': '2px',
        'text-background-shape': 'roundrectangle',
      },
    },
    {
      selector: 'edge.relation-supports',
      css: {
        'line-color': supports,
        'target-arrow-color': supports,
      },
    },
    {
      selector: 'edge.relation-opposes',
      css: {
        'line-color': opposes,
        'target-arrow-color': opposes,
        width: 2.5,
      },
    },
    {
      selector: 'edge.relation-modifies',
      css: {
        'line-color': modifies,
        'target-arrow-color': modifies,
        'line-style': 'dashed',
      },
    },
    {
      selector: 'edge.relation-adopts',
      css: {
        'line-color': adopts,
        'target-arrow-color': adopts,
      },
    },
    {
      selector: 'edge.relation-reinterprets',
      css: {
        'line-color': reinterprets,
        'target-arrow-color': reinterprets,
        'line-style': 'dotted',
      },
    },
    {
      selector: 'edge:selected',
      css: {
        width: 3,
      },
    },
  ];
}
