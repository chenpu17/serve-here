use std::path::Path;

use crate::format::{escape_attr, escape_html, format_bytes, format_date};

pub async fn generate_listing_html(
    request_path: &str,
    dir_path: &Path,
    dashboard_path: &str,
) -> Result<String, std::io::Error> {
    let mut entries: Vec<(String, bool, Option<u64>, Option<std::time::SystemTime>)> = Vec::new();

    let mut read_dir = tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type().await {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        let is_dir = file_type.is_dir();

        let metadata = entry.metadata().await.ok();
        let (size, mtime) = match &metadata {
            Some(m) => {
                let s = if m.is_file() { Some(m.len()) } else { None };
                let t = m.modified().ok();
                (s, t)
            }
            None => (None, None),
        };

        entries.push((name, is_dir, size, mtime));
    }

    entries.sort_by(|a, b| {
        if a.1 && !b.1 {
            std::cmp::Ordering::Less
        } else if !a.1 && b.1 {
            std::cmp::Ordering::Greater
        } else {
            a.0.cmp(&b.0)
        }
    });

    let mut file_count = 0usize;
    let mut directory_count = 0usize;
    let mut total_file_size = 0u64;
    let mut latest_modified: Option<std::time::SystemTime> = None;
    let mut table_rows = String::new();

    for (name, is_dir, size, mtime) in &entries {
        let trailing_slash = if *is_dir { "/" } else { "" };
        let href = urlencoding::encode(name).to_string() + trailing_slash;
        let display_name = format!("{}{}", escape_html(name), trailing_slash);
        let attr_name = escape_attr(name);
        let size_str = if *is_dir {
            "-".to_string()
        } else {
            format_bytes(*size)
        };
        let modified = format_date(*mtime);
        let kind_label = if *is_dir { "Directory" } else { "File" };
        let kind_key = if *is_dir { "kind_directory" } else { "kind_file" };
        let kind_class = if *is_dir { "directory" } else { "file" };
        let meta_label = if *is_dir {
            "Open folder"
        } else {
            "Download or preview file"
        };
        let meta_key = if *is_dir {
            "entry_meta_directory"
        } else {
            "entry_meta_file"
        };
        let icon = if *is_dir {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 6.5A2.5 2.5 0 0 1 5.5 4H10l2 2h6.5A2.5 2.5 0 0 1 21 8.5v9A2.5 2.5 0 0 1 18.5 20h-13A2.5 2.5 0 0 1 3 17.5z" fill="currentColor"/></svg>"#
        } else {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 3.75A1.75 1.75 0 0 1 8.75 2h6.19L21 8.06V20.25A1.75 1.75 0 0 1 19.25 22H8.75A1.75 1.75 0 0 1 7 20.25z" fill="currentColor"/><path d="M14.5 2v5.5H20" fill="none" stroke="rgba(255,255,255,0.55)" stroke-width="1.3"/></svg>"#
        };

        if *is_dir {
            directory_count += 1;
        } else {
            file_count += 1;
            total_file_size += size.unwrap_or(0);
        }

        latest_modified = match (latest_modified, *mtime) {
            (None, Some(value)) => Some(value),
            (Some(current), Some(value)) if value > current => Some(value),
            (current, _) => current,
        };

        table_rows.push_str(&format!(
            r#"<tr class="entry-row" data-name="{}" data-kind="{}">
  <td>
    <a class="entry-link" href="{}">
      <span class="entry-icon {}">{}</span>
      <span class="entry-main">
        <span class="entry-name">{}</span>
        <span class="entry-meta" data-i18n="{}">{}</span>
      </span>
    </a>
  </td>
  <td><span class="kind-badge {}" data-i18n="{}">{}</span></td>
  <td>{}</td>
  <td>{}</td>
</tr>
"#,
            attr_name,
            kind_class,
            href,
            kind_class,
            icon,
            display_name,
            meta_key,
            meta_label,
            kind_class,
            kind_key,
            kind_label,
            size_str,
            modified
        ));
    }

    let parent_row = if request_path != "/" {
        r#"<tr class="entry-row" data-name=".." data-kind="directory">
  <td>
    <a class="entry-link" href="../">
      <span class="entry-icon directory">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M10.5 7 5.5 12l5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/><path d="M6 12h12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
      </span>
      <span class="entry-main">
        <span class="entry-name" data-i18n="parent_directory">Parent directory</span>
        <span class="entry-meta" data-i18n="parent_directory_meta">Step up one level</span>
      </span>
    </a>
  </td>
  <td><span class="kind-badge directory" data-i18n="kind_directory">Directory</span></td>
  <td>-</td>
  <td>-</td>
</tr>"#
    } else {
        ""
    };

    let empty_state = if entries.is_empty() {
        r#"<tr id="emptyStateRow"><td colspan="4" class="empty-state" data-i18n="empty_directory">This directory is empty.</td></tr>"#
    } else {
        r#"<tr id="emptyStateRow" hidden><td colspan="4" class="empty-state" data-i18n="empty_filtered">No entries match the current filter.</td></tr>"#
    };

    let breadcrumbs = build_breadcrumbs(request_path);
    let request_path_attr = escape_attr(request_path);
    let request_path_html = escape_html(request_path);

    let html = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="color-scheme" content="dark light">
    <title>Browse {}</title>
    <script>
      (() => {{
        const memoryStorage = Object.create(null);
        const safeStorage = {{
          get(key) {{
            try {{
              return window.localStorage.getItem(key);
            }} catch (_error) {{
              return Object.prototype.hasOwnProperty.call(memoryStorage, key) ? memoryStorage[key] : null;
            }}
          }},
          set(key, value) {{
            memoryStorage[key] = String(value);
            try {{
              window.localStorage.setItem(key, String(value));
            }} catch (_error) {{
            }}
          }}
        }};
        window.__serveHerePrefs = safeStorage;
        const storedTheme = safeStorage.get("serve-here-theme");
        const storedLang = safeStorage.get("serve-here-lang");
        const theme = storedTheme || (window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark");
        const lang = storedLang || ((navigator.language || "en").toLowerCase().startsWith("zh") ? "zh" : "en");
        document.documentElement.dataset.theme = theme;
        document.documentElement.dataset.lang = lang;
        document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
      }})();
    </script>
    <style>
      :root {{
        color-scheme: dark;
        --bg-start: #07111f;
        --bg-end: #050b14;
        --grid-line: rgba(255, 255, 255, 0.022);
        --panel: rgba(10, 18, 34, 0.82);
        --panel-strong: rgba(14, 26, 49, 0.92);
        --panel-soft: rgba(148, 163, 184, 0.07);
        --line: rgba(148, 163, 184, 0.16);
        --text: #e7eefb;
        --muted: #96abc9;
        --accent: #7c5cff;
        --accent-soft: rgba(124, 92, 255, 0.18);
        --accent-glow: rgba(119, 200, 255, 0.22);
        --accent-contrast: #f8fbff;
        --code-bg: rgba(148, 163, 184, 0.12);
        --code-text: #dff4ff;
        --shadow: 0 24px 64px rgba(0, 0, 0, 0.28);
        --table-head: rgba(6, 12, 24, 0.92);
        --table-hover: rgba(124, 92, 255, 0.08);
      }}

      html[data-theme="light"] {{
        color-scheme: light;
        --bg-start: #f6f8fc;
        --bg-end: #e8eef7;
        --grid-line: rgba(15, 23, 42, 0.04);
        --panel: rgba(255, 255, 255, 0.9);
        --panel-strong: rgba(248, 251, 255, 0.96);
        --panel-soft: rgba(79, 70, 229, 0.05);
        --line: rgba(100, 116, 139, 0.18);
        --text: #13233c;
        --muted: #607089;
        --accent: #4f46e5;
        --accent-soft: rgba(79, 70, 229, 0.1);
        --accent-glow: rgba(14, 165, 233, 0.12);
        --accent-contrast: #ffffff;
        --code-bg: rgba(79, 70, 229, 0.08);
        --code-text: #243b7a;
        --shadow: 0 20px 52px rgba(86, 102, 129, 0.14);
        --table-head: rgba(245, 248, 252, 0.92);
        --table-hover: rgba(79, 70, 229, 0.05);
      }}

      * {{ box-sizing: border-box; }}
      html, body {{ margin: 0; min-height: 100%; }}
      body {{
        font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        font-size: 14px;
        background:
          radial-gradient(circle at top left, rgba(124, 92, 255, 0.18), transparent 24%),
          radial-gradient(circle at top right, var(--accent-glow), transparent 22%),
          linear-gradient(180deg, var(--bg-start) 0%, var(--bg-end) 100%);
        color: var(--text);
        line-height: 1.42;
      }}

      body::before {{
        content: "";
        position: fixed;
        inset: 0;
        background-image:
          linear-gradient(var(--grid-line) 1px, transparent 1px),
          linear-gradient(90deg, var(--grid-line) 1px, transparent 1px);
        background-size: 32px 32px;
        mask-image: linear-gradient(180deg, rgba(0,0,0,0.78), transparent);
        pointer-events: none;
      }}

      a {{ color: inherit; text-decoration: none; }}
      code {{
        padding: 0.14rem 0.42rem;
        border-radius: 999px;
        background: var(--code-bg);
        color: var(--code-text);
      }}

      .shell {{
        width: min(1320px, calc(100vw - 32px));
        margin: 0 auto;
        padding: 14px 0 28px;
      }}

      .hero-card,
      .panel {{
        background: var(--panel);
        border: 1px solid var(--line);
        border-radius: 24px;
        backdrop-filter: blur(18px);
        box-shadow: var(--shadow);
      }}

      .hero {{
        margin-bottom: 12px;
      }}

      .hero-card {{
        padding: 14px;
      }}

      .hero-top {{
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: 14px;
        align-items: start;
      }}

      .eyebrow {{
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 5px 10px;
        border-radius: 999px;
        background: var(--accent-soft);
        color: var(--accent);
        font-size: 10px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.1em;
      }}

      .hero h1 {{
        margin: 8px 0 3px;
        font-size: clamp(1.18rem, 1.55vw, 1.68rem);
        line-height: 1;
        letter-spacing: -0.04em;
      }}

      .hero p {{
        margin: 0;
        max-width: 58ch;
        color: var(--muted);
        font-size: 0.78rem;
      }}

      .hero-side {{
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 8px;
        flex-wrap: wrap;
      }}

      .control-cluster {{
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 4px;
        border-radius: 999px;
        background: var(--panel-soft);
        border: 1px solid var(--line);
      }}

      .control-label {{
        padding-left: 8px;
        color: var(--muted);
        font-size: 0.7rem;
        font-weight: 600;
      }}

      .toggle-group {{
        display: inline-flex;
        gap: 4px;
      }}

      .toggle-button,
      .button {{
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 0;
        cursor: pointer;
        color: inherit;
        font: inherit;
      }}

      .toggle-button {{
        padding: 6px 10px;
        border-radius: 999px;
        background: transparent;
        color: var(--muted);
        font-size: 0.78rem;
        font-weight: 700;
      }}

      .toggle-button.active {{
        background: linear-gradient(135deg, rgba(124, 92, 255, 0.32), rgba(119, 200, 255, 0.18));
        color: var(--text);
      }}

      .hero-actions {{
        display: inline-flex;
        gap: 8px;
        flex-wrap: wrap;
      }}

      .button {{
        padding: 7px 12px;
        border-radius: 999px;
        font-size: 0.82rem;
        font-weight: 700;
        background: linear-gradient(135deg, rgba(124, 92, 255, 0.36), rgba(119, 200, 255, 0.24));
        color: var(--accent-contrast);
        border: 1px solid rgba(180, 194, 255, 0.2);
      }}

      .button.secondary {{
        background: var(--panel-soft);
        color: var(--text);
      }}

      .hero-meta {{
        margin-top: 10px;
        display: grid;
        grid-template-columns: 1.4fr 0.9fr 1fr;
        gap: 8px;
      }}

      .meta-chip {{
        padding: 8px 10px;
        border-radius: 16px;
        background: var(--panel-strong);
        border: 1px solid var(--line);
      }}

      .meta-chip-label {{
        margin-bottom: 4px;
        color: var(--muted);
        font-size: 0.66rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
      }}

      .meta-chip-value {{
        font-size: 0.82rem;
        font-weight: 700;
        line-height: 1.32;
        overflow-wrap: anywhere;
      }}

      .meta-chip-value.path {{
        font-size: 0.74rem;
      }}

      .panel {{
        padding: 14px;
      }}

      .breadcrumb {{
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 7px;
        margin-bottom: 10px;
        color: var(--muted);
      }}

      .crumb {{
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 7px 10px;
        border-radius: 999px;
        background: var(--panel-soft);
        border: 1px solid var(--line);
      }}

      .summary-grid {{
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 8px;
        margin-bottom: 10px;
      }}

      .summary-card {{
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        padding: 10px 12px;
        border-radius: 16px;
        background: var(--panel-soft);
        border: 1px solid var(--line);
      }}

      .summary-card .label {{
        color: var(--muted);
        font-size: 0.72rem;
        font-weight: 600;
      }}

      .summary-card .value {{
        font-size: 0.94rem;
        font-weight: 760;
        letter-spacing: -0.03em;
        text-align: right;
      }}

      .toolbar {{
        display: grid;
        grid-template-columns: minmax(240px, 1fr) auto;
        gap: 10px;
        margin-bottom: 12px;
      }}

      .search {{
        width: 100%;
        border: 1px solid var(--line);
        border-radius: 16px;
        background: var(--panel-soft);
        color: var(--text);
        padding: 11px 13px;
        font: inherit;
      }}

      .search:focus {{
        outline: none;
        border-color: rgba(119, 200, 255, 0.55);
        box-shadow: 0 0 0 4px rgba(119, 200, 255, 0.12);
      }}

      .segmented {{
        display: inline-flex;
        gap: 6px;
        padding: 4px;
        border-radius: 16px;
        background: var(--panel-soft);
        border: 1px solid var(--line);
      }}

      .segmented button {{
        border: 0;
        background: transparent;
        color: var(--muted);
        font: inherit;
        font-size: 0.8rem;
        font-weight: 700;
        padding: 8px 12px;
        border-radius: 12px;
        cursor: pointer;
      }}

      .segmented button.active {{
        background: linear-gradient(135deg, rgba(124, 92, 255, 0.24), rgba(119, 200, 255, 0.16));
        color: var(--text);
      }}

      .table-wrap {{
        overflow: auto;
        border-radius: 20px;
        border: 1px solid var(--line);
        background: var(--panel-soft);
      }}

      table {{
        width: 100%;
        border-collapse: collapse;
        min-width: 760px;
      }}

      th, td {{
        padding: 12px 14px;
        text-align: left;
        border-bottom: 1px solid var(--line);
        vertical-align: middle;
      }}

      thead th {{
        position: sticky;
        top: 0;
        z-index: 1;
        font-size: 0.74rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--muted);
        background: var(--table-head);
        backdrop-filter: blur(12px);
      }}

      tbody tr:hover {{
        background: var(--table-hover);
      }}

      .entry-link {{
        display: flex;
        align-items: center;
        gap: 12px;
      }}

      .entry-icon {{
        width: 38px;
        height: 38px;
        border-radius: 12px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        flex: 0 0 auto;
      }}

      .entry-icon svg {{
        width: 20px;
        height: 20px;
      }}

      .entry-icon.directory {{
        color: #ebf4ff;
        background: linear-gradient(135deg, rgba(124, 92, 255, 0.42), rgba(119, 200, 255, 0.2));
      }}

      .entry-icon.file {{
        color: var(--text);
        background: rgba(148, 163, 184, 0.12);
      }}

      .entry-main {{
        display: grid;
        gap: 2px;
        min-width: 0;
      }}

      .entry-name {{
        font-size: 0.93rem;
        font-weight: 700;
        overflow-wrap: anywhere;
      }}

      .entry-meta {{
        color: var(--muted);
        font-size: 0.77rem;
      }}

      .kind-badge {{
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 6px 10px;
        border-radius: 999px;
        font-size: 0.72rem;
        font-weight: 700;
      }}

      .kind-badge.directory {{
        color: #ece2ff;
        background: rgba(124, 92, 255, 0.16);
      }}

      html[data-theme="light"] .kind-badge.directory {{
        color: #4c1d95;
        background: rgba(124, 92, 255, 0.12);
      }}

      .kind-badge.file {{
        color: #dff4ff;
        background: rgba(119, 200, 255, 0.12);
      }}

      html[data-theme="light"] .kind-badge.file {{
        color: #0f4c75;
        background: rgba(14, 165, 233, 0.12);
      }}

      .empty-state {{
        padding: 26px 14px;
        color: var(--muted);
        text-align: center;
      }}

      .footer {{
        margin-top: 12px;
        color: var(--muted);
        font-size: 0.78rem;
      }}

      .footer a {{
        font-weight: 700;
      }}

      @media (max-width: 1080px) {{
        .hero-top,
        .toolbar {{
          grid-template-columns: 1fr;
        }}

        .hero-side {{
          justify-content: flex-start;
        }}
      }}

      @media (max-width: 980px) {{
        .shell {{
          width: min(100vw - 20px, 100%);
          padding-top: 10px;
        }}

        .hero-meta,
        .summary-grid {{
          grid-template-columns: 1fr;
        }}
      }}

      @media (max-width: 640px) {{
        .hero-card,
        .panel {{
          padding: 12px;
          border-radius: 20px;
        }}

        .summary-card {{
          align-items: flex-start;
          flex-direction: column;
        }}
      }}
    </style>
  </head>
  <body>
    <main class="shell">
      <section class="hero">
        <article class="hero-card">
          <div class="hero-top">
            <div>
              <div class="eyebrow" data-i18n="eyebrow">Directory explorer</div>
              <h1 id="browseHeading" data-request-path="{}">Browse {}</h1>
              <p data-i18n="hero_description">Live directory index with filtering, metadata, and quick navigation.</p>
            </div>
            <div class="hero-side">
              <div class="control-cluster">
                <span class="control-label" data-i18n="theme_label">Theme</span>
                <div class="toggle-group" id="themeToggle">
                  <button type="button" class="toggle-button" data-theme-choice="dark" data-i18n="theme_dark">Dark</button>
                  <button type="button" class="toggle-button" data-theme-choice="light" data-i18n="theme_light">Light</button>
                </div>
              </div>
              <div class="control-cluster">
                <span class="control-label" data-i18n="language_label">Language</span>
                <div class="toggle-group" id="langToggle">
                  <button type="button" class="toggle-button" data-lang-choice="en">EN</button>
                  <button type="button" class="toggle-button" data-lang-choice="zh">中文</button>
                </div>
              </div>
              <div class="hero-actions">
                <a class="button" href="{}" data-role="open-dashboard" data-i18n="open_dashboard">Open stats dashboard</a>
                <a class="button secondary" href="/" data-role="jump-root" data-i18n="jump_to_root">Jump to root</a>
              </div>
            </div>
          </div>
          <div class="hero-meta">
            <div class="meta-chip">
              <div class="meta-chip-label" data-i18n="filesystem_path">Filesystem path</div>
              <div class="meta-chip-value path">{}</div>
            </div>
            <div class="meta-chip">
              <div class="meta-chip-label" data-i18n="current_route">Current route</div>
              <div class="meta-chip-value"><code>{}</code></div>
            </div>
            <div class="meta-chip">
              <div class="meta-chip-label" data-i18n="sorting">Sorting</div>
              <div class="meta-chip-value" data-i18n="sorting_value">Directories first, then alphabetical</div>
            </div>
          </div>
        </article>
      </section>

      <section class="panel">
        <nav class="breadcrumb">{}</nav>

        <section class="summary-grid">
          <article class="summary-card">
            <div class="label" data-i18n="folders">Folders</div>
            <div class="value">{}</div>
          </article>
          <article class="summary-card">
            <div class="label" data-i18n="files">Files</div>
            <div class="value">{}</div>
          </article>
          <article class="summary-card">
            <div class="label" data-i18n="visible_file_size">Visible file size</div>
            <div class="value">{}</div>
          </article>
          <article class="summary-card">
            <div class="label" data-i18n="latest_modified">Latest modified</div>
            <div class="value">{}</div>
          </article>
        </section>

        <section class="toolbar">
          <input id="searchInput" class="search" type="search" data-i18n-placeholder="search_placeholder" placeholder="Filter by file or folder name...">
          <div class="segmented" data-i18n-aria="filter_entries" aria-label="Filter entries">
            <button type="button" class="active" data-filter="all" data-i18n="filter_all">All</button>
            <button type="button" data-filter="directory" data-i18n="filter_folders">Folders</button>
            <button type="button" data-filter="file" data-i18n="filter_files">Files</button>
          </div>
        </section>

        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th data-i18n="table_name">Name</th>
                <th data-i18n="table_type">Type</th>
                <th data-i18n="table_size">Size</th>
                <th data-i18n="table_last_modified">Last modified</th>
              </tr>
            </thead>
            <tbody>
              {}
              {}
              {}
            </tbody>
          </table>
        </div>

        <div class="footer">
          <span id="footerCopy">Served by</span>
          <a href="https://github.com/chenpu17/serve-here" target="_blank" rel="noreferrer">serve-here</a>.
          <span id="footerTail">Files are shown directly from the host filesystem and update on refresh.</span>
        </div>
      </section>
    </main>

    <script>
      const copy = {{
        en: {{
          page_title: "Browse {{path}}",
          browse_heading: "Browse {{path}}",
          eyebrow: "Directory explorer",
          hero_description: "Live directory index with filtering, metadata, and quick navigation.",
          theme_label: "Theme",
          theme_dark: "Dark",
          theme_light: "Light",
          language_label: "Language",
          open_dashboard: "Open stats dashboard",
          jump_to_root: "Jump to root",
          filesystem_path: "Filesystem path",
          current_route: "Current route",
          sorting: "Sorting",
          sorting_value: "Directories first, then alphabetical",
          folders: "Folders",
          files: "Files",
          visible_file_size: "Visible file size",
          latest_modified: "Latest modified",
          search_placeholder: "Filter by file or folder name...",
          filter_entries: "Filter entries",
          filter_all: "All",
          filter_folders: "Folders",
          filter_files: "Files",
          table_name: "Name",
          table_type: "Type",
          table_size: "Size",
          table_last_modified: "Last modified",
          kind_directory: "Directory",
          kind_file: "File",
          entry_meta_directory: "Open folder",
          entry_meta_file: "Download or preview file",
          parent_directory: "Parent directory",
          parent_directory_meta: "Step up one level",
          empty_directory: "This directory is empty.",
          empty_filtered: "No entries match the current filter.",
          breadcrumb_root: "Root",
          footer_copy: "Served by",
          footer_tail: "Files are shown directly from the host filesystem and update on refresh."
        }},
        zh: {{
          page_title: "浏览 {{path}}",
          browse_heading: "浏览 {{path}}",
          eyebrow: "目录浏览器",
          hero_description: "实时目录索引，支持筛选、元数据查看与快速导航。",
          theme_label: "主题",
          theme_dark: "深色",
          theme_light: "亮色",
          language_label: "语言",
          open_dashboard: "打开统计面板",
          jump_to_root: "返回根目录",
          filesystem_path: "文件系统路径",
          current_route: "当前路由",
          sorting: "排序方式",
          sorting_value: "文件夹优先，其后按字母顺序",
          folders: "文件夹",
          files: "文件",
          visible_file_size: "可见文件大小",
          latest_modified: "最近修改",
          search_placeholder: "按文件或文件夹名称筛选...",
          filter_entries: "筛选条目",
          filter_all: "全部",
          filter_folders: "文件夹",
          filter_files: "文件",
          table_name: "名称",
          table_type: "类型",
          table_size: "大小",
          table_last_modified: "最近修改",
          kind_directory: "文件夹",
          kind_file: "文件",
          entry_meta_directory: "打开文件夹",
          entry_meta_file: "下载或预览文件",
          parent_directory: "上级目录",
          parent_directory_meta: "返回上一层",
          empty_directory: "当前目录为空。",
          empty_filtered: "没有符合当前筛选条件的条目。",
          breadcrumb_root: "根目录",
          footer_copy: "由",
          footer_tail: "文件直接从主机文件系统读取，刷新即可看到最新内容。"
        }}
      }};

      const root = document.documentElement;
      const safeStorage = window.__serveHerePrefs || {{
        get: () => null,
        set: () => {{}}
      }};
      const ui = {{
        theme: root.dataset.theme || "dark",
        lang: root.dataset.lang || "en"
      }};

      const browseHeading = document.getElementById("browseHeading");
      const searchInput = document.getElementById("searchInput");
      const themeButtons = Array.from(document.querySelectorAll("[data-theme-choice]"));
      const langButtons = Array.from(document.querySelectorAll("[data-lang-choice]"));
      const filterButtons = Array.from(document.querySelectorAll(".segmented button"));
      const rows = Array.from(document.querySelectorAll(".entry-row"));
      const emptyState = document.getElementById("emptyStateRow");
      let activeFilter = "all";

      function translate(key, vars = {{}}) {{
        const table = copy[ui.lang] || copy.en;
        let value = table[key] || copy.en[key] || key;
        for (const [name, replacement] of Object.entries(vars)) {{
          value = value.replaceAll(`{{${{name}}}}`, replacement);
        }}
        return value;
      }}

      function applyTheme() {{
        root.dataset.theme = ui.theme;
        safeStorage.set("serve-here-theme", ui.theme);
        themeButtons.forEach(button => {{
          button.classList.toggle("active", button.dataset.themeChoice === ui.theme);
        }});
      }}

      function applyLanguage() {{
        root.dataset.lang = ui.lang;
        root.lang = ui.lang === "zh" ? "zh-CN" : "en";
        safeStorage.set("serve-here-lang", ui.lang);

        document.title = translate("page_title", {{ path: browseHeading.dataset.requestPath }});
        browseHeading.textContent = translate("browse_heading", {{ path: browseHeading.dataset.requestPath }});
        document.querySelectorAll("[data-i18n]").forEach(node => {{
          node.textContent = translate(node.dataset.i18n);
        }});
        document.querySelectorAll("[data-i18n-placeholder]").forEach(node => {{
          node.placeholder = translate(node.dataset.i18nPlaceholder);
        }});
        document.querySelectorAll("[data-i18n-aria]").forEach(node => {{
          node.setAttribute("aria-label", translate(node.dataset.i18nAria));
        }});
        document.getElementById("footerCopy").textContent = translate("footer_copy");
        document.getElementById("footerTail").textContent = translate("footer_tail");
        langButtons.forEach(button => {{
          button.classList.toggle("active", button.dataset.langChoice === ui.lang);
        }});
      }}

      function applyFilters() {{
        const query = searchInput.value.trim().toLowerCase();
        let visibleCount = 0;

        rows.forEach(row => {{
          const name = row.dataset.name.toLowerCase();
          const kind = row.dataset.kind;
          const matchesText = !query || name.includes(query);
          const matchesKind = activeFilter === "all" || kind === activeFilter;
          const visible = matchesText && matchesKind;
          row.hidden = !visible;
          if (visible) {{
            visibleCount += 1;
          }}
        }});

        emptyState.hidden = visibleCount !== 0;
      }}

      themeButtons.forEach(button => {{
        button.addEventListener("click", () => {{
          ui.theme = button.dataset.themeChoice;
          applyTheme();
        }});
      }});

      langButtons.forEach(button => {{
        button.addEventListener("click", () => {{
          ui.lang = button.dataset.langChoice;
          applyLanguage();
        }});
      }});

      filterButtons.forEach(button => {{
        button.addEventListener("click", () => {{
          activeFilter = button.dataset.filter;
          filterButtons.forEach(item => item.classList.toggle("active", item === button));
          applyFilters();
        }});
      }});

      searchInput.addEventListener("input", applyFilters);

      applyTheme();
      applyLanguage();
      applyFilters();
    </script>
  </body>
</html>"#,
        request_path_html,
        request_path_attr,
        request_path_html,
        escape_attr(dashboard_path),
        escape_html(&dir_path.display().to_string()),
        request_path_html,
        breadcrumbs,
        directory_count,
        file_count,
        format_bytes(Some(total_file_size)),
        format_date(latest_modified),
        parent_row,
        table_rows.trim_end(),
        empty_state
    );

    Ok(html)
}

fn build_breadcrumbs(request_path: &str) -> String {
    let mut crumbs = vec![r#"<a class="crumb" href="/" data-i18n="breadcrumb_root">Root</a>"#.to_string()];
    let mut current_segments = Vec::new();

    for segment in request_path.split('/').filter(|item| !item.is_empty()) {
        current_segments.push(segment);
        let href = format!(
            "/{}/",
            current_segments
                .iter()
                .map(|item| urlencoding::encode(item).into_owned())
                .collect::<Vec<_>>()
                .join("/")
        );

        crumbs.push(format!(
            r#"<span class="muted">/</span><a class="crumb" href="{}">{}</a>"#,
            href,
            escape_html(segment)
        ));
    }

    crumbs.join("")
}
