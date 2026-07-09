<script lang="ts">
  import { graphData, type Graph, type GraphNode } from "../lib/api";
  import { forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide,
    type Simulation } from "d3-force";
  import { select } from "d3-selection";
  import { zoom, zoomIdentity } from "d3-zoom";
  import { drag } from "d3-drag";
  import { follow } from "../lib/nav.svelte";

  // d3 mutates node objects with x/y/vx/vy; model them loosely.
  type SimNode = GraphNode & { x?: number; y?: number; fx?: number | null; fy?: number | null };
  type SimLink = { source: SimNode; target: SimNode; kind: string };

  let full = $state<Graph>({ nodes: [], edges: [] });
  let focus = $state<string>("CLAUDE.md");
  let hops = $state(1);

  let svgEl: SVGSVGElement;
  let gEl: SVGGElement;
  let sim: Simulation<SimNode, SimLink> | undefined;

  const kindColor: Record<string, string> = {
    claude: "#d97757", file: "#7aa2f7", skill: "#9ece6a", command: "#e0af68",
    agent: "#bb9af7", settings: "#888", script: "#f7768e",
  };

  $effect(() => { graphData().then((g) => (full = g)); });

  // Ego-subgraph: focus node + everything within `hops` (in either direction).
  const ego = $derived.by(() => {
    const adj = new Map<string, Set<string>>();
    for (const e of full.edges) {
      (adj.get(e.from) ?? adj.set(e.from, new Set()).get(e.from)!).add(e.to);
      (adj.get(e.to) ?? adj.set(e.to, new Set()).get(e.to)!).add(e.from);
    }
    const keep = new Set<string>([focus]);
    let frontier = [focus];
    for (let h = 0; h < hops; h++) {
      const next: string[] = [];
      for (const id of frontier) {
        for (const nb of adj.get(id) ?? []) {
          if (!keep.has(nb)) { keep.add(nb); next.push(nb); }
        }
      }
      frontier = next;
    }
    const nodes = full.nodes.filter((n) => keep.has(n.id));
    const edges = full.edges.filter((e) => keep.has(e.from) && keep.has(e.to));
    return { nodes, edges };
  });

  const counts = $derived({
    out: full.edges.filter((e) => e.from === focus).length,
    in: full.edges.filter((e) => e.to === focus).length,
  });

  // Rebuild the simulation whenever the ego-graph changes.
  $effect(() => {
    const data = ego;
    if (!svgEl) return;

    const width = svgEl.clientWidth || 800;
    const height = svgEl.clientHeight || 600;

    const nodes: SimNode[] = data.nodes.map((n) => ({ ...n }));
    const byId = new Map(nodes.map((n) => [n.id, n]));
    const links: SimLink[] = data.edges
      .map((e) => ({ source: byId.get(e.from)!, target: byId.get(e.to)!, kind: e.kind }))
      .filter((l) => l.source && l.target);

    sim?.stop();
    sim = forceSimulation<SimNode>(nodes)
      .force("link", forceLink<SimNode, SimLink>(links).id((d) => d.id).distance(90))
      .force("charge", forceManyBody().strength(-320))
      .force("center", forceCenter(width / 2, height / 2))
      .force("collide", forceCollide(26));

    const g = select(gEl);
    g.selectAll("*").remove();

    const link = g.append("g").attr("stroke-opacity", 0.5)
      .selectAll("line").data(links).join("line")
      .attr("stroke", (d) => (d.kind === "hook" ? "#f7768e" : "#555"))
      .attr("stroke-width", 1.2)
      .attr("marker-end", "url(#arrow)");

    const node = g.append("g").selectAll<SVGGElement, SimNode>("g")
      .data(nodes).join("g").style("cursor", "pointer");

    node.append("circle")
      .attr("r", (d) => (d.id === focus ? 11 : 7))
      .attr("fill", (d) => kindColor[d.kind] ?? "#aaa")
      .attr("stroke", (d) => (d.id === focus ? "#fff" : "#222"))
      .attr("stroke-width", (d) => (d.id === focus ? 2 : 1));

    node.append("text")
      .text((d) => shortLabel(d.id))
      .attr("x", 12).attr("y", 4).attr("fill", "#ccc").attr("font-size", "11px")
      .style("font-family", "ui-monospace, monospace").style("pointer-events", "none");

    node.append("title").text((d) => d.id);

    // Single click re-centers; double click opens in the owning view.
    node.on("click", (_e, d) => { if (d.id !== focus) focus = d.id; })
      .on("dblclick", (_e, d) => follow(d.id));

    node.call(
      drag<SVGGElement, SimNode>()
        .on("start", (event, d) => { if (!event.active) sim!.alphaTarget(0.3).restart(); d.fx = d.x; d.fy = d.y; })
        .on("drag", (event, d) => { d.fx = event.x; d.fy = event.y; })
        .on("end", (event, d) => { if (!event.active) sim!.alphaTarget(0); d.fx = null; d.fy = null; }),
    );

    sim.on("tick", () => {
      link.attr("x1", (d) => d.source.x!).attr("y1", (d) => d.source.y!)
        .attr("x2", (d) => d.target.x!).attr("y2", (d) => d.target.y!);
      node.attr("transform", (d) => `translate(${d.x},${d.y})`);
    });

    // Zoom/pan.
    const z = zoom<SVGSVGElement, unknown>().scaleExtent([0.3, 3])
      .on("zoom", (e) => g.attr("transform", e.transform.toString()));
    select(svgEl).call(z).call(z.transform, zoomIdentity);

    return () => sim?.stop();
  });

  function shortLabel(id: string): string {
    const base = id.split("/").pop() ?? id;
    if (id.endsWith("/SKILL.md")) return id.split("/").at(-2) ?? base; // skill name
    return base.replace(/\.md$/, "");
  }

  // Files the user can focus on (all nodes), for the picker.
  const options = $derived([...full.nodes].sort((a, b) => a.id.localeCompare(b.id)));
</script>

<div class="wrap">
  <div class="bar">
    <label>
      Focus
      <select bind:value={focus}>
        {#each options as n (n.id)}<option value={n.id}>{n.id}</option>{/each}
      </select>
    </label>
    <label>Hops
      <select bind:value={hops}>
        <option value={1}>1</option>
        <option value={2}>2</option>
      </select>
    </label>
    <span class="stat">out {counts.out} · in {counts.in}</span>
    <span class="hint">click node = re-center · double-click = open</span>
  </div>
  <svg bind:this={svgEl} class="canvas">
    <defs>
      <marker id="arrow" viewBox="0 -5 10 10" refX="18" refY="0" markerWidth="6" markerHeight="6" orient="auto">
        <path d="M0,-5L10,0L0,5" fill="#555" />
      </marker>
    </defs>
    <g bind:this={gEl}></g>
  </svg>
</div>

<style>
  .wrap { display: flex; flex-direction: column; height: 100%; }
  .bar {
    display: flex; align-items: center; gap: 14px; padding: 6px 10px;
    border-bottom: 1px solid var(--border); background: var(--bg-alt); font-size: 12px;
  }
  .bar label { color: var(--fg-dim); display: flex; align-items: center; gap: 5px; }
  .stat { color: var(--accent); font-family: ui-monospace, monospace; }
  .hint { margin-left: auto; color: var(--fg-dim); }
  .canvas { flex: 1; width: 100%; min-height: 0; background: var(--bg); }
</style>
