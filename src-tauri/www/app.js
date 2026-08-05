// 变色龙前端：纯 vanilla JS，调用 Tauri 命令薄壳。术语见 CONTEXT.md。
const { invoke } = window.__TAURI__.core;
const HandoffMode = { PARALLEL: "parallel", RELAY: "relay" };

const $ = (id) => document.getElementById(id);
const el = (tag, cls, text) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
};

let toastTimer;
function toast(msg, kind = "") {
  const t = $("toast");
  t.textContent = msg;
  t.className = "toast show " + kind;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.className = "toast"), 2600);
}
async function run(promise, okMsg) {
  try {
    const r = await promise;
    if (okMsg) toast(okMsg, "ok");
    await refresh();
    return r;
  } catch (e) {
    toast(typeof e === "string" ? e : String(e), "err");
    return null;
  }
}

// —— 状态拉取与渲染 ——
async function refresh() {
  const state = await invoke("get_state");
  renderBrowser(state);
  renderRoles(state);
  renderSandboxes(state);
  renderSnapshots(state);
}

function renderBrowser(state) {
  const span = $("browser-path");
  if (state.browser_path) {
    span.textContent = state.browser_path;
    span.classList.remove("muted");
  } else {
    span.textContent = "未找到 Chrome，请手动选择路径";
    span.classList.add("muted");
  }
}

function renderRoles(state) {
  const grid = $("roles");
  grid.innerHTML = "";
  if (state.roles.length === 0) {
    grid.appendChild(el("div", "muted small", "还没有角色，点击右上角「新建角色」开始。"));
    return;
  }
  for (const r of state.roles) {
    const card = el("div", "role-card");
    card.style.setProperty("--role-color", r.color);

    const head = el("div", "head");
    const left = el("div");
    left.appendChild(el("span", "swatch"));
    left.querySelector(".swatch").style.background = r.color;
    left.appendChild(el("span", "name", r.name));
    head.appendChild(left);
    head.appendChild(el("span", "badge " + (r.running ? "running" : ""), r.running ? "运行中" : "未启动"));
    card.appendChild(head);

    const meta = el("div", "port", `端口 ${r.cdp_port} · 数据目录 ${shortDir(r.profile_dir)}`);
    card.appendChild(meta);

    if (r.quickLinks && r.quickLinks.length) {
      const chips = el("div", "preset-chips");
      for (const q of r.quickLinks) {
        const chip = el("span", "chip", q.name);
        chip.title = q.url;
        chip.onclick = () => run(invoke("open_quick_link", { roleId: r.id, name: q.name }), `已在「${r.name}」打开「${q.name}」`);
        chips.appendChild(chip);
      }
      card.appendChild(chips);
    }

    const actions = el("div", "actions");
    if (r.running) {
      actions.appendChild(btn("关闭", "small", () => run(invoke("close_role_cmd", { id: r.id }), `已关闭「${r.name}」`)));
    } else {
      actions.appendChild(btn("启动", "primary small", () => run(invoke("launch_role_cmd", { id: r.id }), `已启动「${r.name}」`)));
    }
    actions.appendChild(btn("接力", "small", () => openHandoff(r)));
    actions.appendChild(btn("预设", "small", () => openLinks(r)));
    actions.appendChild(btn("编辑", "ghost small", () => openRoleDialog(r)));
    actions.appendChild(btn("删除", "danger small", () => deleteRole(r)));
    card.appendChild(actions);

    grid.appendChild(card);
  }
}

function renderSandboxes(state) {
  const box = $("sandboxes");
  box.innerHTML = "";
  if (!state.sandboxes || state.sandboxes.length === 0) {
    box.appendChild(el("div", "muted small", "无运行中的沙箱。"));
    return;
  }
  for (const s of state.sandboxes) {
    const row = el("div", "list-item");
    const meta = el("div", "meta");
    meta.appendChild(el("span", null, "临时沙箱"));
    meta.appendChild(el("small", null, shortDir(s.dir)));
    row.appendChild(meta);
    row.appendChild(btn("关闭并删除", "danger small", () => run(invoke("close_sandbox", { id: s.id }), "沙箱已关闭并清理")));
    box.appendChild(row);
  }
}

function renderSnapshots(state) {
  const box = $("snapshots");
  box.innerHTML = "";
  if (!state.snapshots || state.snapshots.length === 0) {
    box.appendChild(el("div", "muted small", "无快照。保存当前状态以便长流程回归时一键恢复。"));
    return;
  }
  for (const name of state.snapshots) {
    const row = el("div", "list-item");
    const meta = el("div", "meta");
    meta.appendChild(el("span", null, name));
    row.appendChild(meta);
    const acts = el("div");
    acts.appendChild(btn("恢复", "primary small", () => run(invoke("restore_snapshot", { name }), `已恢复快照「${name}」`)));
    acts.appendChild(btn("删除", "ghost small", () => run(invoke("delete_snapshot", { name }), `已删除快照「${name}」`)));
    row.appendChild(acts);
    box.appendChild(row);
  }
}

function btn(label, cls, onclick) {
  const b = el("button", cls, label);
  b.onclick = onclick;
  return b;
}
function shortDir(p) {
  if (!p) return "";
  const parts = String(p).replace(/\\/g, "/").split("/");
  return parts.slice(-2).join("/");
}

// —— 新建/编辑角色 ——
function openRoleDialog(role) {
  const d = $("role-dialog");
  $("role-dialog-title").textContent = role ? "编辑角色" : "新建角色";
  $("role-name").value = role ? role.name : "";
  $("role-color").value = role ? role.color : "#e74c3c";
  d.dataset.id = role ? role.id : "";
  d.showModal();
}
$("btn-new-role").onclick = () => openRoleDialog(null);
$("role-cancel").onclick = () => $("role-dialog").close();
$("role-form").onsubmit = async (e) => {
  e.preventDefault();
  const name = $("role-name").value.trim();
  const color = $("role-color").value;
  const id = $("role-dialog").dataset.id;
  $("role-dialog").close();
  if (id) {
    const existing = (await invoke("get_state")).roles.find((r) => r.id === id);
    await run(invoke("update_role", { role: { ...existing, name, color } }), `已更新「${name}」`);
  } else {
    await run(invoke("create_role", { name, color }), `已创建「${name}」`);
  }
};

async function deleteRole(r) {
  if (!confirm(`确定删除角色「${r.name}」？该角色的数据目录不会被删除，仅移除配置。`)) return;
  await run(invoke("delete_role", { id: r.id }), `已删除角色「${r.name}」`);
}

// —— 接力 ——
let handoffSource = null;
async function openHandoff(source) {
  handoffSource = source;
  const state = await invoke("get_state");
  const sel = $("handoff-target");
  sel.innerHTML = "";
  for (const r of state.roles) {
    if (r.id === source.id) continue;
    sel.appendChild(new Option(r.name, r.id));
  }
  if (sel.options.length === 0) {
    toast("没有其他角色可作为接力目标，请先新建。", "err");
    return;
  }
  $("handoff-source").textContent = source.name;
  $("handoff-url").textContent = "将读取源窗口当前激活标签页 URL";
  $("handoff-dialog").showModal();
}
$("handoff-cancel").onclick = () => $("handoff-dialog").close();
$("handoff-parallel").onclick = () => doHandoff(HandoffMode.PARALLEL);
$("handoff-relay").onclick = () => doHandoff(HandoffMode.RELAY);
async function doHandoff(mode) {
  const targetId = $("handoff-target").value;
  $("handoff-dialog").close();
  const url = await run(
    invoke("handoff_cmd", { sourceId: handoffSource.id, targetId, mode }),
    mode === HandoffMode.RELAY ? "接力完成" : "并行打开完成"
  );
  if (url) toast(`已传递：${url}`, "ok");
}

// —— 预设管理 ——
let linksRole = null;
function openLinks(role) {
  linksRole = role;
  $("links-role").textContent = role.name;
  renderLinksList(role.quickLinks || []);
  $("link-name").value = "";
  $("link-url").value = "";
  $("links-dialog").showModal();
}
function renderLinksList(links) {
  const box = $("links-list");
  box.innerHTML = "";
  if (links.length === 0) box.appendChild(el("div", "muted small", "尚无预设。"));
  for (const q of links) {
    const row = el("div", "list-item");
    const meta = el("div", "meta");
    meta.appendChild(el("span", null, q.name));
    meta.appendChild(el("small", null, q.url));
    row.appendChild(meta);
    row.appendChild(btn("删除", "danger small", () =>
      run(invoke("remove_quick_link", { roleId: linksRole.id, name: q.name }), `已删除预设「${q.name}」`)
    ));
    box.appendChild(row);
  }
}
$("links-close").onclick = () => $("links-dialog").close();
$("links-form").onsubmit = async (e) => {
  e.preventDefault();
  const name = $("link-name").value.trim();
  const url = $("link-url").value.trim();
  await run(invoke("add_quick_link", { roleId: linksRole.id, name, url }), `已添加预设「${name}」`);
  // 刷新当前角色的预设列表
  const state = await invoke("get_state");
  linksRole = state.roles.find((r) => r.id === linksRole.id);
  if (linksRole) renderLinksList(linksRole.quickLinks || []);
  $("link-name").value = "";
  $("link-url").value = "";
};

// —— 浏览器路径 ——
$("btn-pick-browser").onclick = async () => {
  const path = await invoke("pick_browser_path");
  if (!path) return;
  await run(invoke("set_browser_path", { path }), "已保存浏览器路径");
};

// —— 工具栏 ——
$("btn-launch-all").onclick = () =>
  invoke("launch_all").then((r) => {
    toast(`已启动 ${r.ok} 个角色${r.failed ? `，${r.failed} 个失败` : ""}`, r.failed ? "err" : "ok");
    refresh();
  });
$("btn-close-all").onclick = () =>
  invoke("close_all").then((r) => {
    toast(`已关闭 ${r.ok} 个窗口${r.failed ? `，${r.failed} 个失败` : ""}`, r.failed ? "err" : "ok");
    refresh();
  });
$("btn-sandbox").onclick = () =>
  invoke("launch_sandbox").then(() => { toast("已启动临时沙箱", "ok"); refresh(); })
    .catch((e) => toast(String(e), "err"));
$("btn-cleanup").onclick = () =>
  invoke("cleanup_temp").then((n) => { toast(`已清理 ${n} 个临时数据目录`, "ok"); refresh(); })
    .catch((e) => toast(String(e), "err"));
$("btn-export").onclick = async () => {
  const p = await invoke("export_config_cmd");
  if (p) toast(`配置已导出至：${p}`, "ok");
};
$("btn-import").onclick = async () => {
  const n = await invoke("import_config_cmd");
  if (n > 0) toast(`已导入 ${n} 个角色`, "ok");
};

// —— 快照 ——
$("btn-snapshot-save").onclick = async () => {
  const name = $("snapshot-name").value.trim();
  if (!name) { toast("请输入快照名称", "err"); return; }
  await run(invoke("save_snapshot", { name }), `已保存快照「${name}」`);
  $("snapshot-name").value = "";
};

refresh();
