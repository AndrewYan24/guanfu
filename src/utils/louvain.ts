/**
 * Louvain community detection algorithm.
 * Pure TypeScript implementation, no external dependencies.
 */

type EdgeWeight = [string, string, number];

/**
 * Detect communities using the Louvain method.
 * @param nodes - list of node IDs
 * @param edges - list of [source, target] or [source, target, weight]
 * @returns Map from nodeId to clusterId (0-indexed, contiguous)
 */
export function louvain(nodes: string[], edges: EdgeWeight[]): Map<string, number> {
  if (nodes.length === 0) return new Map();
  if (nodes.length === 1) return new Map([[nodes[0], 0]]);

  // Build adjacency with weights (undirected: sum both directions)
  const adj = new Map<string, Map<string, number>>();
  for (const n of nodes) adj.set(n, new Map());
  let totalWeight = 0;

  for (const [u, v, w] of edges) {
    if (u === v) continue;
    const uw = adj.get(u);
    const vw = adj.get(v);
    if (!uw || !vw) continue;
    uw.set(v, (uw.get(v) ?? 0) + w);
    vw.set(u, (vw.get(u) ?? 0) + w);
    totalWeight += w;
  }

  if (totalWeight === 0) {
    // No edges — each node is its own community
    const result = new Map<string, number>();
    nodes.forEach((n, i) => result.set(n, i));
    return result;
  }

  const m2 = 2 * totalWeight;

  // Degree (weighted) for each node
  const degree = new Map<string, number>();
  for (const [n, neighbors] of adj) {
    let sum = 0;
    for (const w of neighbors.values()) sum += w;
    degree.set(n, sum);
  }

  // Community assignment
  const community = new Map<string, number>();
  nodes.forEach((n, i) => community.set(n, i));

  // Sum of degrees within each community
  const sigmaTot = new Map<number, number>(); // sum of degrees of nodes in community
  for (const n of nodes) {
    const c = community.get(n)!;
    sigmaTot.set(c, (sigmaTot.get(c) ?? 0) + (degree.get(n) ?? 0));
  }

  // Phase 1: move nodes to maximize modularity
  function phase1(): boolean {
    let improved = false;
    // Random order to avoid bias
    const order = [...nodes].sort(() => Math.random() - 0.5);

    for (const node of order) {
      const currentC = community.get(node)!;
      const nodeDegree = degree.get(node) ?? 0;
      const neighbors = adj.get(node)!;

      // Remove node from its community
      const oldTot = sigmaTot.get(currentC) ?? 0;
      sigmaTot.set(currentC, oldTot - nodeDegree);

      // Calculate weights to each neighboring community
      const weightsToCommunity = new Map<number, number>();
      for (const [neighbor, w] of neighbors) {
        const nc = community.get(neighbor)!;
        weightsToCommunity.set(nc, (weightsToCommunity.get(nc) ?? 0) + w);
      }

      // Find best community
      let bestC = currentC;
      let bestGain = 0;

      for (const [c, wToC] of weightsToCommunity) {
        const sTot = sigmaTot.get(c) ?? 0;
        // Modularity gain: (w_to_C / m2) - (nodeDegree * sTot / (m2 * m2))
        const gain = wToC / m2 - (nodeDegree * sTot) / (m2 * m2);
        if (gain > bestGain) {
          bestGain = gain;
          bestC = c;
        }
      }

      // Move node to best community
      community.set(node, bestC);
      const newTot = sigmaTot.get(bestC) ?? 0;
      sigmaTot.set(bestC, newTot + nodeDegree);

      if (bestC !== currentC) improved = true;
    }

    return improved;
  }

  // Iteratively improve
  let maxIter = 20;
  while (maxIter-- > 0) {
    if (!phase1()) break;
  }

  // Renumber communities to be contiguous 0..k-1
  const renumber = new Map<number, number>();
  let nextId = 0;
  const result = new Map<string, number>();
  for (const n of nodes) {
    const c = community.get(n)!;
    if (!renumber.has(c)) renumber.set(c, nextId++);
    result.set(n, renumber.get(c)!);
  }

  return result;
}
