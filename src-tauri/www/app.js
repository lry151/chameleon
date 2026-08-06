// chameleon 前端：apple-design + web 交互。
// 字段：读取返回值 snake_case；嵌套发送 role/login snake_case；顶层命令参数 camelCase（Tauri 转）。
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
function btn(label, cls, onclick) {
  const b = el("button", cls, label);
  b.onclick = onclick;
  return b;
}
function shortDir(p) {
  if (!p) return "";
  return String(p).replace(/\\/g, "/").split("/").slice(-2).join("/");
}
let LAST = null;

async function refresh() {
  const state = await invoke("get_state");
  LAST = state;
  renderBrowser(state);
  renderState(state);
}

// —— 浏览器 ——
function renderBrowser(state) {
  const sel = $("browser-select");
  const cur = state.browser_path;
  sel.innerHTML = "";
  let matched = false;
  for (const c of state.browser_candidates) {
    const opt = new Option(`${c.name} — ${c.path}`, c.path);
    if (cur && c.path === cur) matched = true;
    sel.add(opt);
  }
  if (cur && !matched) sel.add(new Option(`当前 — ${cur}`, cur));
  sel.add(new Option("浏览选择文件…", "__browse__"));
  sel.value = cur || (state.browser_candidates[0]?.path) || "__browse__";
  $("browser-status").textContent = state.browser_candidates.length
    ? `检测到 ${state.browser_candidates.length} 个浏览器`
    : "未检测到浏览器，请手动选择";
}
$("browser-select").onchange = async (e) => {
  if (e.target.value === "__browse__") {
    const p = await invoke("pick_browser_path");
    if (p) await run(invoke("set_browser_path", { path: p }), "已保存浏览器路径");
  } else {
    await run(invoke("set_browser_path", { path: e.target.value }), "已切换浏览器");
  }
};
$("btn-pick-browser").onclick = async () => {
  const p = await invoke("pick_browser_path");
  if (p) await run(invoke("set_browser_path", { path: p }), "已保存浏览器路径");
};

// —— 主状态：系统容器嵌套角色卡 ——
function renderState(state) {
  const box = $("state");
  box.innerHTML = "";
  if (!state.systems.length && !state.roles.length) {
    const e = el("div", "panel muted-bg");
    e.appendChild(el("div", "muted small", "还没有系统或角色。点右上角「新建系统」开始——把同一被测应用的多个角色归到一个系统，可批量启动、共享系统级常用 URL。"));
    box.appendChild(e);
    return;
  }
  for (const s of state.systems) {
    box.appendChild(systemBox(s, state));
  }
  const ungrouped = state.roles.filter((r) => !r.system_id);
  if (ungrouped.length) {
    const ub = el("div", "sys-box ungrouped");
    const head = el("div", "sys-head");
    head.appendChild((() => { const d = el("div"); d.appendChild(el("span", "nm", "未分组")); d.appendChild(el("span", "meta", `${ungrouped.length} 角色`)); return d; })());
    ub.appendChild(head);
    const grid = el("div", "role-grid");
    for (const r of ungrouped) grid.appendChild(roleCard(r, state));
    ub.appendChild(grid);
    box.appendChild(ub);
  }
  if (state.sandboxes?.length) box.appendChild(sandboxesPanel(state));
  if (state.snapshots?.length) box.appendChild(snapshotsPanel(state));
}

function systemBox(s, state) {
  const box = el("div", "sys-box");
  const head = el("div", "sys-head");
  const left = el("div");
  left.appendChild(el("span", "nm", s.name));
  const roles = state.roles.filter((r) => r.system_id === s.id);
  left.appendChild(el("span", "meta", `${roles.length} 角色`));
  head.appendChild(left);
  const acts = el("div", "actions");
  acts.appendChild(btn("启动组", "primary small", () =>
    invoke("launch_system", { systemId: s.id }).then((r) => {
      toast(`已启动 ${r.ok} 个角色${r.failed ? `，${r.failed} 个失败` : ""}`, r.failed ? "err" : "ok"); refresh();
    }).catch((e) => toast(String(e), "err"))
  ));
  acts.appendChild(btn("预设", "small", () => openLinks({ kind: "system", id: s.id, name: s.name })));
  acts.appendChild(btn("编辑", "ghost small", () => openSystemDialog(s)));
  acts.appendChild(btn("删除", "danger small", () => { if (confirm(`确定删除系统「${s.name}」？角色保留但变为未分组。`)) run(invoke("delete_system", { id: s.id }), `已删除系统「${s.name}」`); }));
  head.appendChild(acts);
  box.appendChild(head);
  const sysLinks = s.quick_links || [];
  if (sysLinks.length) {
    const chips = el("div", "sys-links");
    for (const q of sysLinks) {
      const c = el("span", "chip", q.name + (q.auto_open ? " ⚡" : ""));
      c.title = q.url;
      const rid = roleInSystem(s.id, state);
      c.onclick = () => rid && run(invoke("open_quick_link", { roleId: rid, name: q.name }), `已打开「${q.name}」`);
      chips.appendChild(c);
    }
    box.appendChild(chips);
  }
  const grid = el("div", "role-grid");
  for (const r of roles) grid.appendChild(roleCard(r, state));
  if (!roles.length) grid.appendChild(el("div", "empty muted small", "系统内还没有角色，点右上角「新建角色」并选此系统"));
  box.appendChild(grid);
  return box;
}

function roleInSystem(sid, state) {
  const r = state.roles.find((x) => x.system_id === sid && x.running) || state.roles.find((x) => x.system_id === sid);
  return r ? r.id : null;
}

function roleCard(r, state) {
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
  card.appendChild(el("div", "port", `端口 ${r.cdp_port} · ${shortDir(r.profile_dir)}`));
  const sysLinks = state.systems.find((s) => s.id === r.system_id)?.quick_links || [];
  const roleLinks = r.quick_links || [];
  if (sysLinks.length || roleLinks.length) {
    const chips = el("div", "preset-chips");
    for (const q of sysLinks) {
      const c = el("span", "chip sys", q.name + (q.auto_open ? " ⚡" : ""));
      c.title = q.url;
      c.onclick = () => run(invoke("open_quick_link", { roleId: r.id, name: q.name }), `已打开「${q.name}」`);
      chips.appendChild(c);
    }
    for (const q of roleLinks) {
      const c = el("span", "chip", q.name + (q.auto_open ? " ⚡" : ""));
      c.style.borderColor = r.color;
      c.title = q.url;
      c.onclick = () => run(invoke("open_quick_link", { roleId: r.id, name: q.name }), `已打开「${q.name}」`);
      chips.appendChild(c);
    }
    card.appendChild(chips);
  }
  const actions = el("div", "actions");
  if (r.running) actions.appendChild(btn("关闭", "small", () => run(invoke("close_role_cmd", { id: r.id }), `已关闭「${r.name}」`)));
  else actions.appendChild(btn("启动", "primary small", () => run(invoke("launch_role_cmd", { id: r.id }), `已启动「${r.name}」`)));
  actions.appendChild(btn("接力", "small", () => openHandoff(r)));
  actions.appendChild(btn("预设", "small", () => openLinks({ kind: "role", id: r.id, name: r.name })));
  if (r.login) actions.appendChild(btn("登录", "accent small", () => run(invoke("login_assist_cmd", { roleId: r.id }), `已为「${r.name}」执行登录辅助`)));
  actions.appendChild(btn("登录配置", "ghost small", () => openLoginDialog(r)));
  actions.appendChild(btn("编辑", "ghost small", () => openRoleDialog(r)));
  actions.appendChild(btn("删除", "danger small", () => deleteRole(r)));
  card.appendChild(actions);
  return card;
}

function sandboxesPanel(state) {
  const p = el("div", "panel");
  p.appendChild((() => { const h = el("div", "panel-head"); h.appendChild(el("h2", null, "临时沙箱")); h.appendChild(el("span", "muted small", "用完即毁，关闭后自动删除数据目录")); return h; })());
  const list = el("div", "list");
  for (const s of state.sandboxes) {
    const row = el("div", "list-item");
    const meta = el("div", "meta");
    meta.appendChild(el("span", null, "临时沙箱"));
    meta.appendChild(el("small", null, shortDir(s.dir)));
    row.appendChild(meta);
    row.appendChild(btn("关闭并删除", "danger small", () => run(invoke("close_sandbox", { id: s.id }), "沙箱已关闭并清理")));
    list.appendChild(row);
  }
  p.appendChild(list);
  return p;
}

function snapshotsPanel(state) {
  const p = el("div", "panel");
  const head = el("div", "panel-head");
  head.appendChild(el("h2", null, "会话快照"));
  head.appendChild(el("span", "muted small", "保存所有角色标签页与窗口位置，长流程回归一键恢复"));
  p.appendChild(head);
  const row = el("div", "list");
  row.style.flexDirection = "row";
  row.style.gap = "8px";
  const inp = el("input"); inp.placeholder = "快照名称"; inp.id = "snapshot-name";
  row.appendChild(inp);
  row.appendChild(btn("保存快照", "primary small", async () => {
    const name = inp.value.trim();
    if (!name) { toast("请输入快照名称", "err"); return; }
    await run(invoke("save_snapshot", { name }), `已保存快照「${name}」`);
    inp.value = "";
  }));
  p.appendChild(row);
  const list = el("div", "list");
  list.style.marginTop = "8px";
  for (const name of state.snapshots) {
    const r = el("div", "list-item");
    const meta = el("div", "meta");
    meta.appendChild(el("span", null, name));
    r.appendChild(meta);
    const a = el("div");
    a.appendChild(btn("恢复", "primary small", () => run(invoke("restore_snapshot", { name }), `已恢复快照「${name}」`)));
    a.appendChild(btn("删除", "ghost small", () => run(invoke("delete_snapshot", { name }), `已删除快照「${name}」`)));
    r.appendChild(a);
    list.appendChild(r);
  }
  p.appendChild(list);
  return p;
}

// —— 系统 dialog ——
$("btn-new-system").onclick = () => openSystemDialog(null);
$("system-cancel").onclick = () => $("system-dialog").close();
$("system-form").onsubmit = async (e) => {
  e.preventDefault();
  const name = $("system-name").value.trim();
  const id = $("system-dialog").dataset.id;
  $("system-dialog").close();
  if (id) await run(invoke("update_system", { system: { ...LAST.systems.find((s) => s.id === id), name } }), `已更新系统「${name}」`);
  else await run(invoke("create_system", { name }), `已创建系统「${name}」`);
};
function openSystemDialog(sys) {
  $("system-dialog-title").textContent = sys ? "编辑系统" : "新建系统";
  $("system-name").value = sys ? sys.name : "";
  $("system-dialog").dataset.id = sys ? sys.id : "";
  $("system-dialog").showModal();
}

// —— 角色 dialog ——
$("btn-new-role").onclick = () => openRoleDialog(null);
$("role-cancel").onclick = () => $("role-dialog").close();
$("role-form").onsubmit = async (e) => {
  e.preventDefault();
  const name = $("role-name").value.trim();
  const color = $("role-color").value;
  const systemId = $("role-system").value || null;
  const id = $("role-dialog").dataset.id;
  $("role-dialog").close();
  if (id) {
    const existing = LAST.roles.find((r) => r.id === id);
    await run(invoke("update_role", { role: { ...existing, name, color, system_id: systemId } }), `已更新「${name}」`);
  } else {
    const created = await run(invoke("create_role", { name, color }), `已创建「${name}」`);
    if (created && systemId) await run(invoke("update_role", { role: { ...created, system_id: systemId } }));
  }
};
function openRoleDialog(role) {
  $("role-dialog-title").textContent = role ? "编辑角色" : "新建角色";
  $("role-name").value = role ? role.name : "";
  $("role-color").value = role ? role.color : "#e74c3c";
  const sel = $("role-system");
  sel.innerHTML = '<option value="">未分组</option>';
  for (const s of LAST.systems) sel.add(new Option(s.name, s.id));
  sel.value = role?.system_id || "";
  $("role-dialog").dataset.id = role ? role.id : "";
  $("role-dialog").showModal();
}
async function deleteRole(r) {
  if (!confirm(`确定删除角色「${r.name}」？数据目录不删，仅移除配置。`)) return;
  await run(invoke("delete_role", { id: r.id }), `已删除角色「${r.name}」`);
}

// —— 接力 ——
let handoffSource = null;
async function openHandoff(source) {
  handoffSource = source;
  const state = await invoke("get_state");
  const sel = $("handoff-target");
  sel.innerHTML = "";
  for (const r of state.roles) { if (r.id === source.id) continue; sel.appendChild(new Option(r.name, r.id)); }
  if (!sel.options.length) { toast("没有其他角色可作为接力目标", "err"); return; }
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
  const url = await run(invoke("handoff_cmd", { sourceId: handoffSource.id, targetId, mode }), mode === HandoffMode.RELAY ? "接力完成" : "并行打开完成");
  if (url) toast(`已传递：${url}`, "ok");
}

// —— 预设管理（角色级 + 系统级，含编辑） ——
let linksOwner = null;
function openLinks(owner) {
  linksOwner = owner;
  $("links-owner").textContent = owner.name;
  renderLinksList();
  $("link-name").value = ""; $("link-url").value = ""; $("link-auto").checked = false;
  $("links-dialog").dataset.editing = "";
  $("links-dialog").showModal();
}
function ownerLinks() {
  if (linksOwner.kind === "system") return LAST.systems.find((s) => s.id === linksOwner.id)?.quick_links || [];
  return LAST.roles.find((r) => r.id === linksOwner.id)?.quick_links || [];
}
function renderLinksList() {
  const box = $("links-list");
  box.innerHTML = "";
  const links = ownerLinks();
  if (!links.length) box.appendChild(el("div", "muted small", "尚无预设。"));
  for (const q of links) {
    const row = el("div", "list-item");
    const meta = el("div", "meta");
    meta.appendChild(el("span", null, q.name + (q.auto_open ? " ⚡" : "")));
    meta.appendChild(el("small", null, q.url));
    row.appendChild(meta);
    const a = el("div");
    a.appendChild(btn("编辑", "small", () => { $("link-name").value = q.name; $("link-url").value = q.url; $("link-auto").checked = q.auto_open; $("links-dialog").dataset.editing = q.name; }));
    a.appendChild(btn("删除", "danger small", () => removeLink(q.name)));
    row.appendChild(a);
    box.appendChild(row);
  }
}
async function removeLink(name) {
  const cmd = linksOwner.kind === "system" ? "remove_system_quick_link" : "remove_quick_link";
  const arg = linksOwner.kind === "system" ? { systemId: linksOwner.id, name } : { roleId: linksOwner.id, name };
  await run(invoke(cmd, arg), `已删除预设「${name}」`);
  LAST = await invoke("get_state");
  renderLinksList();
}
$("links-close").onclick = () => $("links-dialog").close();
$("links-form").onsubmit = async (e) => {
  e.preventDefault();
  const name = $("link-name").value.trim();
  const url = $("link-url").value.trim();
  const autoOpen = $("link-auto").checked;
  const editing = $("links-dialog").dataset.editing;
  if (editing) {
    const cmd = linksOwner.kind === "system" ? "edit_system_quick_link" : "edit_quick_link";
    const arg = linksOwner.kind === "system" ? { systemId: linksOwner.id, oldName: editing, name, url, autoOpen } : { roleId: linksOwner.id, oldName: editing, name, url, autoOpen };
    await run(invoke(cmd, arg), `已更新预设「${name}」`);
  } else {
    const cmd = linksOwner.kind === "system" ? "add_system_quick_link" : "add_quick_link";
    const arg = linksOwner.kind === "system" ? { systemId: linksOwner.id, name, url, autoOpen } : { roleId: linksOwner.id, name, url, autoOpen };
    await run(invoke(cmd, arg), `已添加预设「${name}」`);
  }
  LAST = await invoke("get_state");
  renderLinksList();
  $("link-name").value = ""; $("link-url").value = ""; $("link-auto").checked = false;
  $("links-dialog").dataset.editing = "";
};

// —— 登录辅助配置 ——
let loginRole = null;
function openLoginDialog(role) {
  loginRole = role;
  $("login-role").textContent = role.name;
  const lg = role.login || {};
  $("login-url").value = lg.login_url || "";
  $("login-username").value = lg.username || "";
  $("login-usel").value = lg.username_selector || "";
  $("login-psel").value = lg.password_selector || "";
  $("login-dialog").showModal();
}
$("login-cancel").onclick = () => $("login-dialog").close();
$("login-clear").onclick = async () => { $("login-dialog").close(); await run(invoke("set_role_login", { roleId: loginRole.id, login: null }), `已清除「${loginRole.name}」的登录配置`); };
$("login-form").onsubmit = async (e) => {
  e.preventDefault();
  const login = { login_url: $("login-url").value.trim(), username: $("login-username").value.trim(), username_selector: $("login-usel").value.trim() || null, password_selector: $("login-psel").value.trim() || null };
  $("login-dialog").close();
  await run(invoke("set_role_login", { roleId: loginRole.id, login }), `已保存「${loginRole.name}」的登录配置`);
};

// —— 工具栏全局操作 ——
$("btn-launch-all").onclick = () => invoke("launch_all").then((r) => { toast(`已启动 ${r.ok} 个角色${r.failed ? `，${r.failed} 个失败` : ""}`, r.failed ? "err" : "ok"); refresh(); }).catch((e) => toast(String(e), "err"));
$("btn-close-all").onclick = () => invoke("close_all").then((r) => { toast(`已关闭 ${r.ok} 个窗口${r.failed ? `，${r.failed} 个失败` : ""}`, r.failed ? "err" : "ok"); refresh(); }).catch((e) => toast(String(e), "err"));
$("btn-sandbox").onclick = () => invoke("launch_sandbox").then(() => { toast("已启动临时沙箱", "ok"); refresh(); }).catch((e) => toast(String(e), "err"));
$("btn-cleanup").onclick = () => invoke("cleanup_temp").then((n) => { toast(`已清理 ${n} 个临时目录`, "ok"); refresh(); }).catch((e) => toast(String(e), "err"));
$("btn-export").onclick = async () => { const p = await invoke("export_config_cmd"); if (p) toast(`配置已导出至：${p}`, "ok"); };
$("btn-import").onclick = async () => { const n = await invoke("import_config_cmd"); if (n > 0) toast(`已导入 ${n} 个角色`, "ok"); };

refresh();
