/* Entwine graph enhancement. Progressive: the page is complete without this file.
   No dependencies. Reads #graph-data and animates the already-rendered SVG. */
(() => {
  const app = document.getElementById("graph-app");
  const dataElement = document.getElementById("graph-data");
  if (!app || !dataElement) return;
  let data;
  try {
    data = JSON.parse(dataElement.textContent);
  } catch (_) {
    return;
  }
  const svg = app.querySelector("svg.graph");
  const viewport = svg?.querySelector(".viewport");
  const stage = app.querySelector(".graph-scroll");
  const inspector = app.querySelector(".graph-inspector");
  const controls = app.querySelector(".graph-controls");
  const details = app.closest("details");
  if (!svg || !viewport || !stage || !inspector || !controls) return;

  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const nodes = new Map();
  const elements = new Map();
  svg.querySelectorAll("[data-node]").forEach((el) => {
    elements.set(el.getAttribute("data-node"), el);
  });
  data.nodes.forEach((d) => {
    const el = elements.get(d.id);
    if (!el) return;
    nodes.set(d.id, {
      d,
      el,
      x: d.x,
      y: d.y,
      ox: d.x,
      oy: d.y,
      pinned: false,
      shown: true,
    });
  });
  const edges = [];
  const links = new Map(); // id -> {out:Set, in:Set}
  const link = (id) => {
    if (!links.has(id)) links.set(id, { out: new Set(), in: new Set() });
    return links.get(id);
  };
  svg.querySelectorAll("path.edge").forEach((el) => {
    const s = el.getAttribute("data-source");
    const t = el.getAttribute("data-target");
    if (!nodes.has(s) || !nodes.has(t)) return;
    edges.push({ s, t, el, file: el.classList.contains("edge-file") });
    link(s).out.add(t);
    link(t).in.add(s);
  });
  // Cluster centroids of the generated layout keep the live simulation faithful to it.
  const clusterOf = (n) => (n.d.kind === "documentation" ? n.d.role : n.d.kind);
  const centroids = new Map();
  nodes.forEach((n) => {
    const key = clusterOf(n);
    const c = centroids.get(key) || { x: 0, y: 0, n: 0 };
    c.x += n.ox;
    c.y += n.oy;
    c.n += 1;
    centroids.set(key, c);
  });
  centroids.forEach((c) => {
    c.x /= c.n;
    c.y /= c.n;
  });

  const state = {
    kinds: { repository_file: false },
    roles: {},
    selected: null,
    focus: false,
    view: { x: 0, y: 0, k: 1 },
    alpha: 0,
  };
  app.querySelectorAll("[data-filter-kind]").forEach((box) => {
    state.kinds[box.getAttribute("data-filter-kind")] = box.checked;
  });
  app.querySelectorAll("[data-filter-role]").forEach((box) => {
    state.roles[box.getAttribute("data-filter-role")] = box.checked;
  });

  // ---- geometry --------------------------------------------------------
  const f = (v) => (Math.round(v * 10) / 10).toString();
  const pathFor = (s, t) => {
    const dx = t.x - s.x;
    const dy = t.y - s.y;
    const dist = Math.hypot(dx, dy);
    const sr = s.d.r;
    const tr = t.d.r;
    if (dist < 1) {
      return `M ${f(s.x - 14)} ${f(s.y - sr)} C ${f(s.x - 65)} ${f(s.y - 90)}, ${f(s.x + 65)} ${f(s.y - 90)}, ${f(s.x + 14)} ${f(s.y - tr)}`;
    }
    const bend = dist * 0.12;
    const cx = (s.x + t.x) / 2 - (dy / dist) * bend;
    const cy = (s.y + t.y) / 2 + (dx / dist) * bend;
    const sdx = cx - s.x;
    const sdy = cy - s.y;
    const sd = Math.hypot(sdx, sdy) || 1;
    const edx = t.x - cx;
    const edy = t.y - cy;
    const ed = Math.hypot(edx, edy) || 1;
    return `M ${f(s.x + (sdx / sd) * sr)} ${f(s.y + (sdy / sd) * sr)} Q ${f(cx)} ${f(cy)} ${f(t.x - (edx / ed) * (tr + 4))} ${f(t.y - (edy / ed) * (tr + 4))}`;
  };

  // ---- visibility ------------------------------------------------------
  const neighbors = (id) => {
    const l = links.get(id);
    return l ? new Set([...l.out, ...l.in]) : new Set();
  };
  const isVisible = (n) => {
    if (!state.kinds[n.d.kind]) return false;
    if (n.d.kind === "documentation" && state.roles[n.d.role] === false)
      return false;
    if (state.focus && state.selected) {
      return n.d.id === state.selected || neighbors(state.selected).has(n.d.id);
    }
    return true;
  };
  const refreshVisibility = () => {
    nodes.forEach((n) => {
      n.shown = isVisible(n);
      n.el.classList.toggle("is-off", !n.shown);
    });
    edges.forEach((e) => {
      const on =
        nodes.get(e.s).shown &&
        nodes.get(e.t).shown &&
        (!e.file || state.kinds.repository_file);
      e.on = on;
      e.el.classList.toggle("is-off", !on);
    });
    if (state.selected && !nodes.get(state.selected).shown) select(null);
    paint();
  };

  // ---- painting --------------------------------------------------------
  let queued = false;
  const paint = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      nodes.forEach((n) => {
        if (n.shown)
          n.el.setAttribute("transform", `translate(${f(n.x)} ${f(n.y)})`);
      });
      edges.forEach((e) => {
        if (e.on)
          e.el.setAttribute("d", pathFor(nodes.get(e.s), nodes.get(e.t)));
      });
      const v = state.view;
      viewport.setAttribute(
        "transform",
        `translate(${f(v.x)} ${f(v.y)}) scale(${v.k.toFixed(3)})`,
      );
      svg.classList.toggle("labels-off", v.k < 0.55);
    });
  };

  const size = () => {
    const r = stage.getBoundingClientRect();
    return { w: Math.max(r.width, 1), h: Math.max(r.height, 1) };
  };
  const syncViewBox = () => {
    const { w, h } = size();
    svg.setAttribute("viewBox", `0 0 ${w} ${h}`);
  };
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  let fitted = false;
  const fit = (subset) => {
    const list = [...nodes.values()].filter(
      (n) => n.shown && (!subset || subset.has(n.d.id)),
    );
    if (!list.length) return;
    const xs = list.map((n) => n.x);
    const ys = list.map((n) => n.y);
    const pad = 70;
    const minX = Math.min(...xs) - pad;
    const maxX = Math.max(...xs) + pad;
    const minY = Math.min(...ys) - pad;
    const maxY = Math.max(...ys) + pad + 20;
    const { w, h } = size();
    if (w < 60 || h < 60) return; // not laid out yet (collapsed or loading); retried on resize
    fitted = true;
    const k = clamp(Math.min(w / (maxX - minX), h / (maxY - minY)), 0.12, 1.6);
    state.view = {
      k,
      x: w / 2 - ((minX + maxX) / 2) * k,
      y: h / 2 - ((minY + maxY) / 2) * k,
    };
    paint();
  };
  const toGraph = (clientX, clientY) => {
    const r = stage.getBoundingClientRect();
    const v = state.view;
    return {
      x: (clientX - r.left - v.x) / v.k,
      y: (clientY - r.top - v.y) / v.k,
    };
  };
  const zoomAt = (factor, cx, cy) => {
    const v = state.view;
    const k = clamp(v.k * factor, 0.1, 3);
    const real = k / v.k;
    state.view = { k, x: cx - (cx - v.x) * real, y: cy - (cy - v.y) * real };
    paint();
  };
  const zoomCentered = (factor) => {
    const { w, h } = size();
    zoomAt(factor, w / 2, h / 2);
  };
  const centerOn = (n) => {
    const { w, h } = size();
    const k = clamp(state.view.k, 0.8, 1.4);
    state.view = { k, x: w / 2 - n.x * k, y: h / 2 - n.y * k };
    paint();
  };

  // ---- simulation (same forces as the generated layout, damped) ---------
  let running = false;
  const wake = (alpha) => {
    if (reduced) return;
    state.alpha = Math.max(state.alpha, alpha);
    if (!running) {
      running = true;
      requestAnimationFrame(step);
    }
  };
  const step = () => {
    const live = [...nodes.values()].filter((n) => n.shown);
    const force = new Map(live.map((n) => [n.d.id, { x: 0, y: 0 }]));
    for (let i = 0; i < live.length; i++) {
      for (let j = i + 1; j < live.length; j++) {
        const a = live[i];
        const b = live[j];
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let d2 = dx * dx + dy * dy;
        if (d2 > 250000) continue;
        if (d2 < 4) {
          dx = 1 + (i % 3);
          dy = 1 + (j % 3);
          d2 = dx * dx + dy * dy;
        }
        const d = Math.sqrt(d2);
        const push = 27225 / d;
        const fa = force.get(a.d.id);
        const fb = force.get(b.d.id);
        fa.x += (dx / d) * push;
        fa.y += (dy / d) * push;
        fb.x -= (dx / d) * push;
        fb.y -= (dy / d) * push;
      }
    }
    edges.forEach((e) => {
      if (!e.on) return;
      const a = nodes.get(e.s);
      const b = nodes.get(e.t);
      const dx = a.x - b.x;
      const dy = a.y - b.y;
      const d = Math.max(Math.hypot(dx, dy), 0.01);
      const pull = ((d * d) / 165) * 0.85;
      const fa = force.get(e.s);
      const fb = force.get(e.t);
      fa.x -= (dx / d) * pull;
      fa.y -= (dy / d) * pull;
      fb.x += (dx / d) * pull;
      fb.y += (dy / d) * pull;
    });
    live.forEach((n) => {
      const c = centroids.get(clusterOf(n));
      const fr = force.get(n.d.id);
      if (c) {
        fr.x += (c.x - n.x) * 0.12;
        fr.y += (c.y - n.y) * 0.12;
      }
      if (n.pinned || n === dragging) return;
      const m = Math.hypot(fr.x, fr.y);
      if (m > 0) {
        const move = Math.min(m * 0.04, 14) * state.alpha;
        n.x += (fr.x / m) * move;
        n.y += (fr.y / m) * move;
      }
    });
    state.alpha *= dragging ? 0.995 : 0.955;
    paint();
    if (state.alpha > 0.02) requestAnimationFrame(step);
    else running = false;
  };

  // ---- selection & inspector -------------------------------------------
  const el = (tag, text, className) => {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    if (className) node.className = className;
    return node;
  };
  const neighborList = (title, ids) => {
    const wrap = el("div", undefined, "graph-neighbors");
    wrap.appendChild(el("h3", `${title} (${ids.length})`));
    if (!ids.length) {
      wrap.appendChild(el("p", "None.", "graph-empty"));
      return wrap;
    }
    const list = el("ul");
    ids.forEach((id) => {
      const n = nodes.get(id);
      if (!n) return;
      const item = el("li");
      const button = el("button", n.d.label);
      button.type = "button";
      button.addEventListener("click", () => select(id, true));
      item.appendChild(button);
      item.appendChild(el("span", ` ${n.d.kind_label}`, "graph-kind"));
      list.appendChild(item);
    });
    wrap.appendChild(list);
    return wrap;
  };
  const renderInspector = () => {
    inspector.textContent = "";
    const n = state.selected && nodes.get(state.selected);
    if (!n) {
      inspector.appendChild(
        el(
          "p",
          "Select a node to see its details and neighbors.",
          "graph-empty",
        ),
      );
      return;
    }
    const d = n.d;
    inspector.appendChild(el("h2", d.label));
    const badges = el("p", undefined, "graph-badges");
    badges.appendChild(el("span", d.kind_label));
    if (d.kind === "documentation" && d.role !== "other")
      badges.appendChild(el("span", d.role_label));
    inspector.appendChild(badges);
    const facts = el("dl");
    const fact = (name, value) => {
      facts.appendChild(el("dt", name));
      facts.appendChild(el("dd", value));
    };
    fact("Source", d.path);
    if (d.scope !== null && d.scope !== undefined)
      fact("Scope", d.scope === "" ? "Project root" : d.scope);
    if (d.kind === "repository_file")
      fact("Referenced by", String(d.referenced_by.length));
    else
      fact(
        "Links",
        `${d.out} out · ${d.in} in · ${d.refs} repository file${d.refs === 1 ? "" : "s"}`,
      );
    inspector.appendChild(facts);
    if (d.href) {
      const open = el("a", "Open page");
      open.href = d.href;
      open.className = "graph-open";
      inspector.appendChild(open);
    }
    const l = links.get(d.id) || { out: new Set(), in: new Set() };
    if (d.kind === "repository_file") {
      inspector.appendChild(neighborList("Referenced by", d.referenced_by));
    } else {
      inspector.appendChild(
        neighborList(
          "References",
          [...l.out].filter((id) => nodes.get(id).d.kind !== "repository_file"),
        ),
      );
      inspector.appendChild(neighborList("Referenced by", [...l.in]));
      const files = [...l.out].filter(
        (id) => nodes.get(id).d.kind === "repository_file",
      );
      if (files.length) {
        const wrap = el("div", undefined, "graph-neighbors");
        wrap.appendChild(el("h3", `Repository files (${files.length})`));
        const list = el("ul");
        for (const id of files)
          list.appendChild(el("li", nodes.get(id).d.path));
        wrap.appendChild(list);
        inspector.appendChild(wrap);
      }
    }
  };
  const focusButton = controls.querySelector('[data-action="focus"]');
  function select(id, center) {
    state.selected = id && nodes.has(id) ? id : null;
    if (!state.selected) state.focus = false;
    const near = state.selected ? neighbors(state.selected) : new Set();
    svg.classList.toggle("has-selection", !!state.selected);
    nodes.forEach((n) => {
      n.el.classList.toggle("is-selected", n.d.id === state.selected);
      n.el.classList.toggle("is-neighbor", near.has(n.d.id));
    });
    edges.forEach((e) => {
      e.el.classList.toggle(
        "is-active",
        !!state.selected && (e.s === state.selected || e.t === state.selected),
      );
    });
    if (focusButton) {
      focusButton.disabled = !state.selected;
      focusButton.setAttribute("aria-pressed", String(state.focus));
    }
    renderInspector();
    try {
      const url = new URL(location.href);
      if (state.selected) url.searchParams.set("focus", state.selected);
      else url.searchParams.delete("focus");
      history.replaceState(null, "", url);
    } catch (_) {
      /* file: or sandboxed contexts */
    }
    refreshVisibility();
    if (state.selected && center) centerOn(nodes.get(state.selected));
    if (state.focus) fit();
  }

  // ---- pointer interaction ---------------------------------------------
  let dragging = null;
  let dragMoved = false;
  let panning = null;
  const pointers = new Map();
  let pinch = 0;
  svg.addEventListener("dragstart", (e) => e.preventDefault());
  svg.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    svg.setPointerCapture(e.pointerId);
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      pinch = Math.hypot(a.x - b.x, a.y - b.y);
      dragging = null;
      panning = null;
      return;
    }
    const target = e.target.closest?.(".graph-node");
    dragMoved = false;
    if (target && nodes.has(target.getAttribute("data-node"))) {
      dragging = nodes.get(target.getAttribute("data-node"));
      dragging.start = { x: e.clientX, y: e.clientY };
    } else {
      panning = {
        x: e.clientX,
        y: e.clientY,
        vx: state.view.x,
        vy: state.view.y,
      };
    }
  });
  svg.addEventListener("pointermove", (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      const distance = Math.hypot(a.x - b.x, a.y - b.y);
      if (pinch > 0) {
        const r = stage.getBoundingClientRect();
        zoomAt(
          distance / pinch,
          (a.x + b.x) / 2 - r.left,
          (a.y + b.y) / 2 - r.top,
        );
      }
      pinch = distance;
      return;
    }
    if (dragging) {
      if (
        !dragMoved &&
        Math.hypot(e.clientX - dragging.start.x, e.clientY - dragging.start.y) <
          4
      )
        return;
      dragMoved = true;
      const p = toGraph(e.clientX, e.clientY);
      dragging.x = p.x;
      dragging.y = p.y;
      dragging.pinned = true;
      dragging.el.classList.add("is-pinned");
      wake(0.45);
      paint();
    } else if (panning) {
      state.view.x = panning.vx + (e.clientX - panning.x);
      state.view.y = panning.vy + (e.clientY - panning.y);
      if (Math.hypot(e.clientX - panning.x, e.clientY - panning.y) > 3)
        dragMoved = true;
      paint();
    }
  });
  const release = (e) => {
    pointers.delete(e.pointerId);
    pinch = 0;
    if (e.type === "pointerup" && pointers.size === 0) {
      if (dragging && !dragMoved) select(dragging.d.id, false);
      else if (!dragging && panning && !dragMoved) select(null);
    }
    if (pointers.size === 0) {
      dragging = null;
      panning = null;
    }
  };
  svg.addEventListener("pointerup", release);
  svg.addEventListener("pointercancel", release);
  svg.addEventListener("click", (e) => {
    const target = e.target.closest?.(".graph-node");
    if (!target) return;
    e.preventDefault();
    // Keyboard activation (Enter on a focused node) has no pointer sequence.
    if (e.detail === 0) select(target.getAttribute("data-node"), true);
  });
  svg.addEventListener("dblclick", (e) => {
    const target = e.target.closest?.(".graph-node");
    const n = target && nodes.get(target.getAttribute("data-node"));
    if (n?.d.href) location.href = n.d.href;
  });
  stage.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      const r = stage.getBoundingClientRect();
      zoomAt(
        Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0015)),
        e.clientX - r.left,
        e.clientY - r.top,
      );
    },
    { passive: false },
  );
  stage.addEventListener("keydown", (e) => {
    const move = {
      ArrowLeft: [40, 0],
      ArrowRight: [-40, 0],
      ArrowUp: [0, 40],
      ArrowDown: [0, -40],
    }[e.key];
    if (e.target.closest?.(".graph-node") && move) return;
    if (move) {
      state.view.x += move[0];
      state.view.y += move[1];
      paint();
    } else if (e.key === "+" || e.key === "=") zoomCentered(1.25);
    else if (e.key === "-") zoomCentered(0.8);
    else if (e.key === "0") fit();
    else if (e.key === "Escape") select(null);
    else return;
    e.preventDefault();
  });

  // ---- controls ---------------------------------------------------------
  for (const box of controls.querySelectorAll("[data-filter-kind]")) {
    box.addEventListener("change", () => {
      state.kinds[box.getAttribute("data-filter-kind")] = box.checked;
      refreshVisibility();
      fit();
    });
  }
  for (const box of controls.querySelectorAll("[data-filter-role]")) {
    box.addEventListener("change", () => {
      state.roles[box.getAttribute("data-filter-role")] = box.checked;
      refreshVisibility();
      fit();
    });
  }
  controls.addEventListener("click", (e) => {
    const button = e.target.closest?.("[data-action]");
    if (!button) return;
    const action = button.getAttribute("data-action");
    if (action === "zoom-in") zoomCentered(1.3);
    else if (action === "zoom-out") zoomCentered(1 / 1.3);
    else if (action === "fit") fit();
    else if (action === "focus") {
      state.focus = !state.focus;
      button.setAttribute("aria-pressed", String(state.focus));
      refreshVisibility();
      fit();
    } else if (action === "reset") {
      state.alpha = 0;
      const from = [...nodes.values()].map((n) => [n, n.x, n.y]);
      from.forEach(([n]) => {
        n.pinned = false;
        n.el.classList.remove("is-pinned");
      });
      const started = performance.now();
      const run = (now) => {
        const t = reduced ? 1 : Math.min(1, (now - started) / 380);
        const ease = 1 - (1 - t) ** 3;
        from.forEach(([n, x, y]) => {
          n.x = x + (n.ox - x) * ease;
          n.y = y + (n.oy - y) * ease;
        });
        paint();
        if (t < 1) requestAnimationFrame(run);
        else fit();
      };
      requestAnimationFrame(run);
    }
  });

  // ---- start ------------------------------------------------------------
  app.classList.add("is-live");
  svg.removeAttribute("width");
  svg.removeAttribute("height");
  svg.setAttribute("preserveAspectRatio", "xMidYMid meet");
  controls.hidden = false;
  inspector.hidden = false;
  stage.removeAttribute("tabindex");
  stage.setAttribute("tabindex", "0");
  nodes.forEach((n) => {
    n.el.setAttribute("tabindex", "0");
  });
  syncViewBox();
  refreshVisibility();
  fit();
  const params = new URLSearchParams(location.search);
  const wanted = params.get("focus");
  if (wanted) {
    let found = nodes.get(wanted);
    if (!found) found = [...nodes.values()].find((n) => n.d.path === wanted);
    if (found) {
      if (!state.kinds[found.d.kind]) {
        state.kinds[found.d.kind] = true;
        const box = controls.querySelector(
          `[data-filter-kind="${found.d.kind}"]`,
        );
        if (box) box.checked = true;
      }
      refreshVisibility();
      select(found.d.id, true);
      const near = neighbors(found.d.id);
      near.add(found.d.id);
      fit(near);
      if (state.view.k > 1.2) centerOn(found);
    }
  }
  const resize = () => {
    syncViewBox();
    if (!fitted) fit();
    paint();
  };
  if (window.ResizeObserver) new ResizeObserver(resize).observe(stage);
  else window.addEventListener("resize", resize);
  if (details)
    details.addEventListener("toggle", () => {
      if (!details.open) return;
      resize();
      fit();
    });
})();
