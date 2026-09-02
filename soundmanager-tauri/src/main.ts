/**
 * Sound Manager frontend — compact table view, native dialogs, catalog, downloader.
 */
import { invoke } from "@tauri-apps/api/core";
import { open as dialogOpen, save as dialogSave } from "@tauri-apps/plugin-dialog";
import { ipc } from "./app/ipc";
import { loadLanguage, apply, t, type Lang } from "./app/i18n";

type ViewKey = "scheme" | "catalog" | "downloader" | "settings" | "about";
const $ = <T extends HTMLElement = HTMLElement>(sel: string): T => {
  const el = document.querySelector<T>(sel);
  if (!el) throw new Error(`missing: ${sel}`);
  return el;
};
function toast(msg: string, ms = 2600): void {
  const el = $("#toast");
  el.textContent = msg;
  el.hidden = false;
  window.setTimeout(() => (el.hidden = true), ms);
}
function switchView(view: ViewKey): void {
  document.querySelectorAll<HTMLElement>(".view").forEach((v) => v.classList.remove("active"));
  $(`#view-${view}`).classList.add("active");
  document.querySelectorAll<HTMLElement>(".nav-btn").forEach((b) =>
    b.classList.toggle("active", (b as HTMLElement).dataset.view === view)
  );
}
$("#nav").addEventListener("click", (e) => {
  const btn = (e.target as HTMLElement).closest<HTMLElement>(".nav-btn");
  if (btn?.dataset.view) switchView(btn.dataset.view as ViewKey);
});

// ── Scheme (compact table) ──
async function refreshEvents(): Promise<void> {
  const events = await ipc.getSoundEvents();
  const rows = $("#event-rows");
  rows.replaceChildren();
  for (const ev of events) {
    const row = document.createElement("div");
    row.className = `event-row${ev.disabled ? " disabled" : ""}`;
    row.dataset.event = ev.internal_name;

    const icon = document.createElement("div");
    icon.className = "ev-icon";
    icon.textContent = "♪";

    const nameCol = document.createElement("div");
    const name = document.createElement("div");
    name.className = "ev-name";
    name.textContent = ev.display_name;
    const desc = document.createElement("div");
    desc.className = "ev-desc";
    desc.textContent = ev.description;
    nameCol.append(name, desc);

    const file = document.createElement("div");
    file.className = `ev-file${ev.has_file ? "" : " missing"}`;
    file.textContent = ev.file_name ?? t("no_image");
    file.title = ev.file_name ?? "";

    const actions = document.createElement("div");
    actions.className = "ev-actions";
    const mk = (label: string, title: string, onClick: () => void): HTMLButtonElement => {
      const b = document.createElement("button");
      b.className = "icon-btn";
      b.textContent = label;
      b.title = title;
      b.addEventListener("click", onClick);
      return b;
    };
    actions.append(
      mk("▶", t("button_play"), async () => { try { await ipc.playSoundEvent(ev.internal_name); } catch (e) { toast(String(e)); } }),
      mk("…", t("button_open"), async () => {
        const path = await dialogOpen({ filters: [{ name: "Audio", extensions: ["wav", "mp3", "ogg", "flac", "m4a"] }] });
        if (!path) return;
        const p = Array.isArray(path) ? path[0] : path as string;
        try { await ipc.updateSoundFile(ev.internal_name, p); await refreshEvents(); } catch (e) { toast(String(e)); }
      }),
      mk("↺", t("button_reset"), async () => { try { await ipc.removeSoundFile(ev.internal_name); await refreshEvents(); } catch (e) { toast(String(e)); } }),
      mk("📁", "Open folder", async () => { try { await invoke("open_sound_location", { eventInternal: ev.internal_name }); } catch (e) { toast(String(e)); } })
    );

    const toggle = document.createElement("input");
    toggle.type = "checkbox";
    toggle.className = "ev-toggle";
    toggle.checked = !ev.disabled;
    toggle.title = t("button_enable");
    toggle.addEventListener("change", async () => {
      try { await ipc.setEventDisabled(ev.internal_name, !toggle.checked); await refreshEvents(); } catch (e) { toast(String(e)); }
    });

    row.append(icon, nameCol, file, actions, toggle);
    rows.append(row);
  }
}

async function refreshMeta(): Promise<void> {
  const meta = await ipc.getSchemeMeta();
  ($("#meta-name") as HTMLInputElement).value = meta.name;
  ($("#meta-author") as HTMLInputElement).value = meta.author;
  ($("#meta-about") as HTMLInputElement).value = meta.about;
  const thumb = $("#scheme-thumb") as HTMLImageElement;
  thumb.src = meta.thumbnail_base64 ? `data:image/png;base64,${meta.thumbnail_base64}` : "data:,";
}
async function saveMeta(): Promise<void> {
  await invoke("set_scheme_meta", { meta: {
    name: ($("#meta-name") as HTMLInputElement).value,
    author: ($("#meta-author") as HTMLInputElement).value,
    about: ($("#meta-about") as HTMLInputElement).value,
    thumbnail_base64: currentThumbB64 || (await ipc.getSchemeMeta()).thumbnail_base64,
  }});
}
let currentThumbB64 = "";
$("#meta-name").addEventListener("change", () => saveMeta().catch((e) => toast(String(e))));
$("#meta-author").addEventListener("change", () => saveMeta().catch((e) => toast(String(e))));
$("#meta-about").addEventListener("change", () => saveMeta().catch((e) => toast(String(e))));
$("#thumb-file").addEventListener("change", async () => {
  const file = ($("#thumb-file") as HTMLInputElement).files?.[0];
  if (!file) return;
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  currentThumbB64 = btoa(binary);
  try { await saveMeta(); await refreshMeta(); } catch (e) { toast(String(e)); }
});

// Import / export / reset (native dialogs)
$("#btn-import").addEventListener("click", async () => {
  const path = await dialogOpen({ filters: [{ name: "Sound scheme", extensions: ["ths", "soundpack", "zip"] }] });
  if (!path) return;
  const p = Array.isArray(path) ? path[0] : path as string;
  try { await ipc.importArchive(p); await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]); toast(t("import_done")); } catch (e) { toast(String(e)); }
});
$("#btn-export").addEventListener("click", async () => {
  const path = await dialogSave({ defaultPath: "MyScheme.ths", filters: [{ name: "Sound scheme", extensions: ["ths"] }] });
  if (!path) return;
  try { await ipc.exportArchive(path); toast(t("export_done")); } catch (e) { toast(String(e)); }
});
$("#btn-reset").addEventListener("click", async () => {
  try { await ipc.setupSchemeManager(true); await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]); toast(t("reset_done")); } catch (e) { toast(String(e)); }
});

// ── Scheme list (sidebar) ──
async function refreshSchemeList(): Promise<void> {
  const [schemes, active] = await Promise.all([ipc.getSchemeList(), ipc.getActiveScheme()]);
  const sel = $("#scheme-select") as HTMLSelectElement;
  sel.replaceChildren();
  for (const s of schemes) {
    const opt = document.createElement("option");
    opt.value = s.internal_name;
    opt.textContent = s.display_name;
    if (active && s.internal_name === active.internal_name) opt.selected = true;
    sel.append(opt);
  }
  await refreshImportSystemSchemes();
}
$("#scheme-select").addEventListener("change", async (e) => {
  const sel = e.target as HTMLSelectElement;
  try {
    const settings = await ipc.getSettings();
    await ipc.applyScheme(sel.value, settings.missing_sound_use_default);
    toast(t("scheme_applied"));
  } catch (err) { toast(String(err)); }
});

// ── Settings ──
async function loadSettingsView(): Promise<void> {
  const s = await ipc.getSettings();
  ($("#set-patch") as HTMLInputElement).checked = s.patch_startup_sound;
  ($("#set-default-missing") as HTMLInputElement).checked = s.missing_sound_use_default;
  ($("#set-convert") as HTMLInputElement).checked = s.convert_proprietary_files;
  ($("#set-prefer-startup") as HTMLInputElement).checked = s.prefer_startup_sound_on_logon;
  ($("#set-lang") as HTMLSelectElement).value = s.language;
  try {
    const assoc = await invoke<boolean>("get_file_association");
    ($("#set-fileassoc") as HTMLInputElement).checked = assoc;
  } catch {}
  try {
    const info = await invoke<{ friendly_name: string; nt_version: string; supported: boolean }>("get_system_info");
    const el = document.getElementById("about-system");
    if (el) el.textContent = `${info.friendly_name} / Windows NT ${info.nt_version} — ${info.supported ? "supported" : "unsupported"}`;
  } catch {}
}

async function refreshImportSystemSchemes(): Promise<void> {
  const sel = document.getElementById("import-system-select") as HTMLSelectElement | null;
  if (!sel) return;
  try {
    const schemes = await ipc.getSchemeList();
    // keep first option
    sel.replaceChildren(sel.options[0]);
    for (const s of schemes) {
      if (s.internal_name === ".Default" || s.internal_name === "SoundManager") continue;
      const o = document.createElement("option");
      o.value = s.internal_name;
      o.textContent = s.display_name;
      sel.append(o);
    }
  } catch {}
}
$("#btn-save-settings").addEventListener("click", async () => {
  const s = await ipc.getSettings();
  const patchWasOn = s.patch_startup_sound;
  const patchNow = ($("#set-patch") as HTMLInputElement).checked;
  const next = {
    ...s,
    patch_startup_sound: patchNow,
    missing_sound_use_default: ($("#set-default-missing") as HTMLInputElement).checked,
    convert_proprietary_files: ($("#set-convert") as HTMLInputElement).checked,
    prefer_startup_sound_on_logon: ($("#set-prefer-startup") as HTMLInputElement).checked,
    language: ($("#set-lang") as HTMLSelectElement).value,
  };
  try {
    if (patchNow && !patchWasOn) await ipc.patchStartupSound(true);
    else if (!patchNow && patchWasOn) await ipc.restoreStartupSound();
    await ipc.saveSettings(next);
    // file association (separate, needs no restart)
    try {
      const wantAssoc = ($("#set-fileassoc") as HTMLInputElement).checked;
      await invoke("set_file_association", { associated: wantAssoc });
    } catch (e) { toast(String(e)); }
    await loadLanguage(next.language as Lang);
    toast(t("settings_saved"));
  } catch (e) { toast(String(e)); await loadSettingsView(); }
});

// ── Catalog (MyInstants) ──
const previewAudio = $("#preview-audio") as HTMLAudioElement;
let catalogResults: Array<{ title: string; url: string; page_url: string }> = [];

async function doCatalogSearch(): Promise<void> {
  const q = ($("#catalog-query") as HTMLInputElement).value.trim();
  if (!q) return;
  const status = $("#catalog-status");
  const grid = $("#catalog-grid");
  status.textContent = t("catalog_searching") ?? "Searching…";
  grid.replaceChildren();
  try {
    catalogResults = await invoke<Array<{ title: string; url: string; page_url: string }>>("search_catalog", { query: q });
    if (catalogResults.length === 0) status.textContent = t("catalog_no_results") ?? "No results.";
    else status.textContent = `${catalogResults.length} results`;
    const events = await ipc.getSoundEvents();
    for (const r of catalogResults) {
      const card = document.createElement("div");
      card.className = "catalog-card";
      const title = document.createElement("div");
      title.className = "catalog-card-title";
      title.textContent = r.title;
      title.title = r.title;
      const actions = document.createElement("div");
      actions.className = "catalog-card-actions";
      const play = document.createElement("button");
      play.className = "btn small";
      play.textContent = "▶";
      play.title = "Preview";
      play.addEventListener("click", () => { previewAudio.src = r.url; previewAudio.play().catch(() => {}); });
      const sel = document.createElement("select");
      sel.className = "btn small";
      sel.style.flex = "1";
      const ph = document.createElement("option");
      ph.textContent = t("catalog_assign") ?? "Assign to…";
      ph.disabled = true; ph.selected = true;
      sel.append(ph);
      for (const ev of events) {
        const o = document.createElement("option");
        o.value = ev.internal_name;
        o.textContent = ev.display_name;
        sel.append(o);
      }
      sel.addEventListener("change", async () => {
        const ev = sel.value;
        sel.disabled = true;
        try {
          await invoke("assign_catalog_sound", { url: r.url, eventInternal: ev, title: r.title });
          await refreshEvents();
          toast(`${r.title} → ${ev}`);
        } catch (e) { toast(String(e)); }
        sel.disabled = false;
        sel.selectedIndex = 0;
      });
      actions.append(play, sel);
      card.append(title, actions);
      grid.append(card);
    }
  } catch (e) { status.textContent = String(e); }
}
$("#catalog-search-btn").addEventListener("click", doCatalogSearch);
$("#catalog-query").addEventListener("keydown", (e) => { if (e.key === "Enter") doCatalogSearch(); });

// ── GitHub downloader ──
async function doGhSearch(): Promise<void> {
  const q = ($("#gh-search") as HTMLInputElement).value.trim();
  const status = $("#gh-status");
  const list = $("#schemes-list");
  status.textContent = "Searching…";
  list.replaceChildren();
  try {
    const schemes = await invoke<Array<{ name: string; download_url: string; size: number }>>("search_github_schemes", { query: q || undefined });
    if (schemes.length === 0) status.textContent = "No schemes found.";
    else status.textContent = `${schemes.length} schemes`;
    for (const s of schemes) {
      const card = document.createElement("div");
      card.className = "catalog-card";
      const t2 = document.createElement("div");
      t2.className = "catalog-card-title";
      t2.textContent = s.name;
      const btn = document.createElement("button");
      btn.className = "btn small primary";
      btn.textContent = t("button_import");
      btn.addEventListener("click", async () => {
        btn.disabled = true;
        try {
          await invoke("download_github_scheme", { downloadUrl: s.download_url, fileName: s.name });
          await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]);
          toast(t("import_done"));
        } catch (e) { toast(String(e)); }
        btn.disabled = false;
      });
      card.append(t2, btn);
      list.append(card);
    }
  } catch (e) { status.textContent = String(e); }
}
$("#gh-search-btn").addEventListener("click", doGhSearch);
$("#gh-search").addEventListener("keydown", (e) => { if (e.key === "Enter") doGhSearch(); });

// ── About ──
$("#link-repo").addEventListener("click", (e) => { e.preventDefault(); window.open("https://github.com/Israleche/Sound-Manager", "_blank"); });
$("#link-help")?.addEventListener("click", (e) => { e.preventDefault(); window.open("https://github.com/Israleche/Sound-Manager#readme", "_blank"); });
$("#link-website")?.addEventListener("click", (e) => { e.preventDefault(); window.open("https://github.com/ORelio/Sound-Manager", "_blank"); });

// Import system scheme
document.getElementById("import-system-select")?.addEventListener("change", async (e) => {
  const sel = e.target as HTMLSelectElement;
  if (!sel.value) return;
  try {
    await invoke("import_system_scheme", { internalName: sel.value });
    await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]);
    toast(t("import_done"));
  } catch (err) { toast(String(err)); }
  sel.selectedIndex = 0;
});

// Maintenance
document.getElementById("btn-reinstall")?.addEventListener("click", async () => {
  try { await invoke("reinstall_app"); toast("Reinstall triggered"); } catch (e) { toast(String(e)); }
});
document.getElementById("btn-config")?.addEventListener("click", async () => {
  try { await invoke("reveal_config_file"); } catch (e) { toast(String(e)); }
});
document.getElementById("btn-uninstall")?.addEventListener("click", async () => {
  if (!confirm("Uninstall Sound Manager integration? This removes the scheme and closes the app.")) return;
  try { await invoke("uninstall_app"); } catch (e) { toast(String(e)); }
});
document.getElementById("btn-themepack")?.addEventListener("click", async () => {
  const path = await dialogSave({ defaultPath: "MyScheme.themepack", filters: [{ name: "Themepack", extensions: ["themepack"] }] });
  if (!path) return;
  try { await invoke("export_themepack", { destination: path }); toast(t("export_done")); } catch (e) { toast(String(e)); }
});

// Drag & drop (import .ths or replace sound)
document.addEventListener("dragover", (e) => { e.preventDefault(); });
document.addEventListener("drop", async (e) => {
  e.preventDefault();
  const files = (e as DragEvent).dataTransfer?.files;
  if (!files || files.length === 0) return;
  // Tauri drag-drop gives file paths via dataTransfer; fallback to File.path
  const first = files[0] as File & { path?: string };
  const p = first.path ?? first.name;
  const ext = p.split(".").pop()?.toLowerCase();
  if (ext === "ths" || ext === "soundpack" || ext === "zip") {
    try { await ipc.importArchive(p); await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]); toast(t("import_done")); } catch (err) { toast(String(err)); }
  }
});

// ── Boot ──
async function boot(): Promise<void> {
  const info = await ipc.getAppInfo();
  $("#about-version").textContent = `Sound Manager ${info.version} — ${info.windows_friendly}`;
  await loadLanguage(info.language as Lang);
  await loadSettingsView();
  try { await ipc.setupSchemeManager(false); } catch { /* first run */ }
  await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList(), refreshImportSystemSchemes()]);
  apply();
}
boot().catch((e) => { console.error(e); toast(String(e), 8000); });
