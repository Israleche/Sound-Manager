/**
 * Sound Manager frontend entrypoint: boot, i18n, view switching, scheme view,
 * settings view, import/export. Vanilla TS + DOM, no framework.
 */
import { ipc } from "./app/ipc";
import { loadLanguage, apply, t, type Lang } from "./app/i18n";

type ViewKey = "scheme" | "downloader" | "settings" | "about";

const $ = <T extends HTMLElement = HTMLElement>(sel: string): T => {
  const el = document.querySelector<T>(sel);
  if (!el) throw new Error(`missing element: ${sel}`);
  return el;
};

function toast(msg: string, ms = 2600): void {
  const el = $("#toast");
  el.textContent = msg;
  el.hidden = false;
  window.setTimeout(() => (el.hidden = true), ms);
}

// ------------------------------------------------- Navigation

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

// ------------------------------------------------- Scheme view

async function refreshEvents(): Promise<void> {
  const events = await ipc.getSoundEvents();
  const grid = $("#event-grid");
  grid.replaceChildren();
  for (const ev of events) {
    const card = document.createElement("div");
    card.className = `event-card${ev.disabled ? " disabled" : ""}`;
    card.dataset.event = ev.internal_name;

    const info = document.createElement("div");
    info.className = "event-info";
    const name = document.createElement("div");
    name.className = "event-name";
    name.textContent = ev.display_name;
    const desc = document.createElement("div");
    desc.className = "event-desc";
    desc.textContent = ev.description;
    const file = document.createElement("div");
    file.className = `event-file${ev.has_file ? "" : " missing"}`;
    file.textContent = ev.file_name ?? t("no_image");
    info.append(name, desc, file);

    const actions = document.createElement("div");
    actions.className = "event-actions";

    const mkBtn = (label: string, title: string, onClick: () => void): HTMLButtonElement => {
      const b = document.createElement("button");
      b.className = "icon-btn";
      b.textContent = label;
      b.title = title;
      b.addEventListener("click", onClick);
      return b;
    };

    actions.append(
      mkBtn("▶", t("button_play"), async () => {
        try {
          await ipc.playSoundEvent(ev.internal_name);
        } catch (e) {
          toast(String(e));
        }
      }),
      mkBtn("📄", t("button_open"), async () => {
        const input = document.createElement("input");
        input.type = "file";
        input.accept = ".wav,.mp3,.ogg,.flac,.m4a,audio/*";
        input.addEventListener("change", async () => {
          const f = input.files?.[0];
          if (!f) return;
          try {
            // Tauri fs scope: pass the path we got from the file dialog plugin
            const path = (f as File & { path?: string }).path ?? f.name;
            await ipc.updateSoundFile(ev.internal_name, path);
            await refreshEvents();
          } catch (e) {
            toast(String(e));
          }
        });
        input.click();
      }),
      mkBtn("↺", t("button_reset"), async () => {
        try {
          await ipc.removeSoundFile(ev.internal_name);
          await refreshEvents();
        } catch (e) {
          toast(String(e));
        }
      })
    );

    const toggle = document.createElement("input");
    toggle.type = "checkbox";
    toggle.checked = !ev.disabled;
    toggle.title = t("button_enable");
    toggle.addEventListener("change", async () => {
      try {
        await ipc.setEventDisabled(ev.internal_name, !toggle.checked);
        await refreshEvents();
      } catch (e) {
        toast(String(e));
      }
    });
    actions.append(toggle);

    card.append(info, actions);
    grid.append(card);
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
  await ipc.setSchemeMeta({
    name: ($("#meta-name") as HTMLInputElement).value,
    author: ($("#meta-author") as HTMLInputElement).value,
    about: ($("#meta-about") as HTMLInputElement).value,
    thumbnail_base64: currentThumbB64,
  });
}

let currentThumbB64 = "";

$("#meta-name").addEventListener("change", () => saveMeta().catch(toast));
$("#meta-author").addEventListener("change", () => saveMeta().catch(toast));
$("#meta-about").addEventListener("change", () => saveMeta().catch(toast));

$("#thumb-file").addEventListener("change", async () => {
  const file = ($("#thumb-file") as HTMLInputElement).files?.[0];
  if (!file) return;
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  currentThumbB64 = btoa(binary);
  await saveMeta();
  await refreshMeta();
});

// ------------------------------------------------- Import / export / reset

$("#btn-import").addEventListener("click", async () => {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".ths,.soundpack,.theme,.zip";
  input.addEventListener("change", async () => {
    const f = input.files?.[0];
    if (!f) return;
    try {
      const path = (f as File & { path?: string }).path ?? f.name;
      await ipc.importArchive(path);
      await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]);
      toast(t("import_done"));
    } catch (e) {
      toast(String(e));
    }
  });
  input.click();
});

$("#btn-export").addEventListener("click", async () => {
  const input = document.createElement("input");
  input.type = "file";
  // nwsaveas: non-standard but supported in Chromium-based save pickers
  input.setAttribute("nwsaveas", "MyScheme.ths");
  input.accept = ".ths";
  input.addEventListener("change", async () => {
    const f = input.files?.[0];
    if (!f) return;
    try {
      const path = (f as File & { path?: string }).path ?? f.name;
      await ipc.exportArchive(path);
      toast(t("export_done"));
    } catch (e) {
      toast(String(e));
    }
  });
  input.click();
});

$("#btn-reset").addEventListener("click", async () => {
  try {
    await ipc.setupSchemeManager(true);
    await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]);
    toast(t("reset_done"));
  } catch (e) {
    toast(String(e));
  }
});

// ------------------------------------------------- Scheme list (sidebar)

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
}

$("#scheme-select").addEventListener("change", async (e) => {
  const sel = e.target as HTMLSelectElement;
  try {
    const settings = await ipc.getSettings();
    await ipc.applyScheme(sel.value, settings.missing_sound_use_default);
    toast(t("scheme_applied"));
  } catch (err) {
    toast(String(err));
  }
});

// ------------------------------------------------- Settings view

async function loadSettingsView(): Promise<void> {
  const s = await ipc.getSettings();
  ($("#set-patch") as HTMLInputElement).checked = s.patch_startup_sound;
  ($("#set-default-missing") as HTMLInputElement).checked = s.missing_sound_use_default;
  ($("#set-convert") as HTMLInputElement).checked = s.convert_proprietary_files;
  ($("#set-prefer-startup") as HTMLInputElement).checked = s.prefer_startup_sound_on_logon;
  ($("#set-listview") as HTMLInputElement).checked = s.scheme_items_list_view;
  ($("#set-lang") as HTMLSelectElement).value = s.language;
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
    scheme_items_list_view: ($("#set-listview") as HTMLInputElement).checked,
    language: ($("#set-lang") as HTMLSelectElement).value,
  };
  try {
    // imageres patching requires admin: attempt, but never silently swallow failure
    if (patchNow && !patchWasOn) {
      await ipc.patchStartupSound(true);
    } else if (!patchNow && patchWasOn) {
      await ipc.restoreStartupSound();
    }
    await ipc.saveSettings(next);
    document.body.classList.toggle("listview", next.scheme_items_list_view);
    await loadLanguage(next.language as Lang);
    toast(t("settings_saved"));
  } catch (e) {
    toast(String(e));
    await loadSettingsView();
  }
});

// ------------------------------------------------- About

$("#link-repo").addEventListener("click", (e) => {
  e.preventDefault();
  window.open("https://github.com/Israleche/Sound-Manager", "_blank");
});

// ------------------------------------------------- Boot

async function boot(): Promise<void> {
  const info = await ipc.getAppInfo();
  $("#about-version").textContent = `Sound Manager ${info.version} — ${info.windows_friendly}`;
  await loadLanguage(info.language as Lang);
  await loadSettingsView();
  document.body.classList.toggle("listview", ($("#set-listview") as HTMLInputElement).checked);
  try {
    await ipc.setupSchemeManager(false);
  } catch {
    // first run may fail without media dir; setupSchemeManager(true) from UI handles it
  }
  await Promise.all([refreshEvents(), refreshMeta(), refreshSchemeList()]);
  apply();
}

boot().catch((e) => {
  console.error(e);
  toast(String(e), 8000);
});
