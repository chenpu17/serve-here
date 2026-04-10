use crate::format::{escape_attr, escape_html, format_url_host};

pub fn render_stats_page(
    root_dir: &str,
    host: &str,
    port: u16,
    started_at_ms: u64,
    dashboard_data_path: &str,
) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="color-scheme" content="dark light">
    <title>serve-here stats</title>
    <script>
      (() => {{
        const storedTheme = localStorage.getItem("serve-here-theme");
        const storedLang = localStorage.getItem("serve-here-lang");
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
        --bg-start: #08111f;
        --bg-end: #050b15;
        --grid-line: rgba(255, 255, 255, 0.024);
        --panel: rgba(11, 19, 36, 0.82);
        --panel-strong: rgba(17, 28, 48, 0.92);
        --panel-soft: rgba(148, 163, 184, 0.07);
        --line: rgba(148, 163, 184, 0.16);
        --text: #e8eefc;
        --muted: #9eb1cf;
        --accent: #77c8ff;
        --accent-strong: #7c5cff;
        --accent-soft: rgba(119, 200, 255, 0.14);
        --accent-glow: rgba(124, 92, 255, 0.2);
        --accent-contrast: #f8fbff;
        --code-bg: rgba(148, 163, 184, 0.12);
        --code-text: #dff4ff;
        --shadow: 0 24px 64px rgba(0, 0, 0, 0.3);
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
        --accent: #0f77c7;
        --accent-strong: #5b56df;
        --accent-soft: rgba(14, 165, 233, 0.1);
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
        line-height: 1.42;
        background:
          radial-gradient(circle at top left, rgba(124, 92, 255, 0.16), transparent 22%),
          radial-gradient(circle at top right, var(--accent-glow), transparent 20%),
          linear-gradient(180deg, var(--bg-start) 0%, var(--bg-end) 100%);
        color: var(--text);
      }}

      body::before {{
        content: "";
        position: fixed;
        inset: 0;
        background-image:
          linear-gradient(var(--grid-line) 1px, transparent 1px),
          linear-gradient(90deg, var(--grid-line) 1px, transparent 1px);
        background-size: 32px 32px;
        mask-image: linear-gradient(180deg, rgba(0, 0, 0, 0.8), transparent);
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
        width: min(1440px, calc(100vw - 32px));
        margin: 0 auto;
        padding: 14px 0 28px;
      }}

      .hero-card,
      .panel,
      .summary-card {{
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

      .pulse {{
        width: 8px;
        height: 8px;
        border-radius: 999px;
        background: linear-gradient(135deg, #4ade80, #bef264);
        box-shadow: 0 0 0 0 rgba(74, 222, 128, 0.34);
        animation: pulse 1.8s infinite;
      }}

      @keyframes pulse {{
        0% {{ box-shadow: 0 0 0 0 rgba(74, 222, 128, 0.34); }}
        70% {{ box-shadow: 0 0 0 12px rgba(74, 222, 128, 0); }}
        100% {{ box-shadow: 0 0 0 0 rgba(74, 222, 128, 0); }}
      }}

      .hero h1 {{
        margin: 8px 0 3px;
        font-size: clamp(1.24rem, 1.7vw, 1.9rem);
        line-height: 1;
        letter-spacing: -0.04em;
      }}

      .hero p {{
        margin: 0;
        max-width: 60ch;
        color: var(--muted);
        font-size: 0.8rem;
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

      .summary-grid {{
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 8px;
        margin-bottom: 12px;
      }}

      .summary-card {{
        padding: 10px 12px;
      }}

      .summary-label {{
        color: var(--muted);
        font-size: 0.72rem;
        font-weight: 600;
      }}

      .summary-value {{
        margin-top: 6px;
        font-size: 1.02rem;
        font-weight: 760;
        letter-spacing: -0.03em;
      }}

      .summary-note {{
        margin-top: 4px;
        color: var(--muted);
        font-size: 0.74rem;
      }}

      .content-grid {{
        display: grid;
        grid-template-columns: minmax(0, 1.35fr) minmax(320px, 0.82fr);
        gap: 12px;
      }}

      .panel {{
        padding: 14px;
      }}

      .panel + .panel {{
        margin-top: 12px;
      }}

      .panel-header {{
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        margin-bottom: 12px;
      }}

      .panel h2 {{
        margin: 0;
        font-size: 1rem;
        letter-spacing: -0.02em;
      }}

      .panel-subtitle {{
        margin-top: 4px;
        color: var(--muted);
        font-size: 0.76rem;
      }}

      .muted {{
        color: var(--muted);
      }}

      .resource-grid,
      .chart-grid {{
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 8px;
      }}

      .resource-grid {{
        margin-bottom: 8px;
      }}

      .resource-card,
      .chart-card {{
        padding: 10px 12px;
        border-radius: 18px;
        background: var(--panel-soft);
        border: 1px solid var(--line);
      }}

      .resource-label,
      .mini-caption,
      .footer-note {{
        color: var(--muted);
        font-size: 0.74rem;
      }}

      .resource-value {{
        margin-top: 6px;
        font-size: 0.98rem;
        font-weight: 760;
        letter-spacing: -0.03em;
      }}

      .resource-bar,
      .meter {{
        margin-top: 8px;
        height: 8px;
        border-radius: 999px;
        overflow: hidden;
        background: rgba(148, 163, 184, 0.12);
      }}

      .resource-bar > span,
      .meter span {{
        display: block;
        height: 100%;
        border-radius: inherit;
        background: linear-gradient(90deg, var(--accent), var(--accent-strong));
      }}

      .chart-title {{
        margin-bottom: 8px;
        font-size: 0.84rem;
        font-weight: 700;
      }}

      svg {{
        display: block;
        width: 100%;
        height: auto;
      }}

      .pill-list {{
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }}

      .pill {{
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 8px 10px;
        border-radius: 999px;
        background: var(--panel-soft);
        border: 1px solid var(--line);
      }}

      .list-stack {{
        display: grid;
        gap: 10px;
      }}

      .list-row {{
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: 10px;
        align-items: center;
      }}

      .list-label {{
        min-width: 0;
      }}

      .list-label strong {{
        display: block;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }}

      .request-table-wrap {{
        overflow: auto;
        border-radius: 20px;
        border: 1px solid var(--line);
        background: var(--panel-soft);
      }}

      table {{
        width: 100%;
        border-collapse: collapse;
        min-width: 820px;
      }}

      th, td {{
        text-align: left;
        padding: 12px 14px;
        border-bottom: 1px solid var(--line);
        vertical-align: top;
      }}

      thead th {{
        position: sticky;
        top: 0;
        background: var(--table-head);
        backdrop-filter: blur(12px);
        font-size: 0.74rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--muted);
      }}

      tbody tr:hover {{
        background: var(--table-hover);
      }}

      .status-badge,
      .method-badge {{
        display: inline-flex;
        min-width: 54px;
        justify-content: center;
        padding: 6px 9px;
        border-radius: 999px;
        font-size: 0.72rem;
        font-weight: 700;
      }}

      .method-badge {{
        background: rgba(119, 200, 255, 0.12);
        color: #d5f3ff;
      }}

      html[data-theme="light"] .method-badge {{
        color: #0f4c75;
        background: rgba(14, 165, 233, 0.12);
      }}

      .status-2xx {{ background: rgba(74, 222, 128, 0.14); color: #d6ffe6; }}
      .status-3xx {{ background: rgba(119, 200, 255, 0.14); color: #d6f4ff; }}
      .status-4xx {{ background: rgba(251, 191, 36, 0.14); color: #ffe7a8; }}
      .status-5xx {{ background: rgba(251, 113, 133, 0.14); color: #ffd3db; }}

      html[data-theme="light"] .status-2xx {{ color: #166534; }}
      html[data-theme="light"] .status-3xx {{ color: #0f4c75; }}
      html[data-theme="light"] .status-4xx {{ color: #92400e; }}
      html[data-theme="light"] .status-5xx {{ color: #9f1239; }}

      @media (max-width: 1180px) {{
        .hero-top,
        .content-grid,
        .chart-grid {{
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
        .summary-grid,
        .resource-grid {{
          grid-template-columns: 1fr;
        }}
      }}

      @media (max-width: 640px) {{
        .hero-card,
        .panel,
        .summary-card {{
          padding: 12px;
          border-radius: 20px;
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
              <div class="eyebrow"><span class="pulse"></span><span data-i18n="eyebrow">Live runtime observatory</span></div>
              <h1 data-i18n="hero_title">Service control deck</h1>
              <p data-i18n="hero_description">A live operational view of request traffic, route activity, client reach, and resource pressure for the current <code>serve-here</code> process.</p>
            </div>
            <div class="hero-side">
              <div class="control-cluster">
                <span class="control-label" data-i18n="theme_label">Theme</span>
                <div class="toggle-group">
                  <button type="button" class="toggle-button" data-theme-choice="dark" data-i18n="theme_dark">Dark</button>
                  <button type="button" class="toggle-button" data-theme-choice="light" data-i18n="theme_light">Light</button>
                </div>
              </div>
              <div class="control-cluster">
                <span class="control-label" data-i18n="language_label">Language</span>
                <div class="toggle-group">
                  <button type="button" class="toggle-button" data-lang-choice="en">EN</button>
                  <button type="button" class="toggle-button" data-lang-choice="zh">中文</button>
                </div>
              </div>
              <div class="hero-actions">
                <a class="button" href="/" data-role="browse-files" data-i18n="browse_files">Browse files</a>
                <a class="button secondary" href="https://github.com/chenpu17/serve-here" target="_blank" rel="noreferrer" data-i18n="project_repo">Project repo</a>
              </div>
            </div>
          </div>
          <div class="hero-meta">
            <div class="meta-chip">
              <div class="meta-chip-label" data-i18n="served_directory">Served directory</div>
              <div class="meta-chip-value">{}</div>
            </div>
            <div class="meta-chip">
              <div class="meta-chip-label" data-i18n="bind_address">Bind address</div>
              <div class="meta-chip-value">http://{}:{}</div>
            </div>
            <div class="meta-chip">
              <div class="meta-chip-label" data-i18n="dashboard_started">Dashboard started</div>
              <div class="meta-chip-value" id="startedAt" data-started-at-ms="{}">Loading...</div>
            </div>
          </div>
        </article>
      </section>

      <section class="summary-grid" id="summaryCards"></section>

      <section class="content-grid">
        <div>
          <section class="panel">
            <div class="panel-header">
              <div>
                <h2 data-i18n="resource_pressure">Resource pressure</h2>
                <div class="panel-subtitle" data-i18n="resource_pressure_subtitle">Process and machine health refreshed every five seconds.</div>
              </div>
              <div class="muted" id="refreshState" data-i18n="refreshing">Refreshing...</div>
            </div>
            <div class="resource-grid" id="resourceCards"></div>
            <div class="chart-grid">
              <div class="chart-card">
                <div class="chart-title" data-i18n="process_cpu">Process CPU</div>
                <div id="cpuChart"></div>
                <div class="mini-caption" id="cpuCaption" data-i18n="waiting_for_data">Waiting for data...</div>
              </div>
              <div class="chart-card">
                <div class="chart-title" data-i18n="process_memory">Process memory</div>
                <div id="memoryChart"></div>
                <div class="mini-caption" id="memoryCaption" data-i18n="waiting_for_data">Waiting for data...</div>
              </div>
            </div>
          </section>

          <section class="panel">
            <div class="panel-header">
              <div>
                <h2 data-i18n="latest_requests">Latest requests</h2>
                <div class="panel-subtitle" data-i18n="latest_requests_subtitle">Newest 1000 requests, excluding the dashboard JSON polling endpoint.</div>
              </div>
              <div class="muted" id="requestCount">0 captured</div>
            </div>
            <div class="request-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th data-i18n="table_time">Time</th>
                    <th data-i18n="table_method">Method</th>
                    <th data-i18n="table_status">Status</th>
                    <th data-i18n="table_path">Path</th>
                    <th data-i18n="table_client">Client</th>
                    <th data-i18n="table_latency">Latency</th>
                    <th data-i18n="table_bytes">Bytes</th>
                  </tr>
                </thead>
                <tbody id="recentRequests"></tbody>
              </table>
            </div>
          </section>
        </div>

        <div>
          <section class="panel">
            <div class="panel-header">
              <div>
                <h2 data-i18n="top_routes">Top routes</h2>
                <div class="panel-subtitle" data-i18n="top_routes_subtitle">Most frequently requested paths in this process lifetime.</div>
              </div>
            </div>
            <div class="list-stack" id="topRoutes"></div>
          </section>

          <section class="panel">
            <div class="panel-header">
              <div>
                <h2 data-i18n="client_footprint">Client footprint</h2>
                <div class="panel-subtitle" data-i18n="client_footprint_subtitle">Unique client IPs and their request volume.</div>
              </div>
            </div>
            <div class="list-stack" id="topClients"></div>
          </section>

          <section class="panel">
            <div class="panel-header">
              <div>
                <h2 data-i18n="request_mix">Request mix</h2>
                <div class="panel-subtitle" data-i18n="request_mix_subtitle">Method and status distribution across observed traffic.</div>
              </div>
            </div>
            <div class="pill-list" id="methodPills"></div>
            <div class="footer-note" data-i18n="method_distribution">HTTP method distribution</div>
            <div style="height: 14px"></div>
            <div class="pill-list" id="statusPills"></div>
            <div class="footer-note" data-i18n="status_distribution">Response status distribution</div>
          </section>
        </div>
      </section>
    </main>

    <script>
      const copy = {{
        en: {{
          page_title: "serve-here stats",
          eyebrow: "Live runtime observatory",
          hero_title: "Service control deck",
          hero_description: "A live operational view of request traffic, route activity, client reach, and resource pressure for the current serve-here process.",
          theme_label: "Theme",
          theme_dark: "Dark",
          theme_light: "Light",
          language_label: "Language",
          browse_files: "Browse files",
          project_repo: "Project repo",
          served_directory: "Served directory",
          bind_address: "Bind address",
          dashboard_started: "Dashboard started",
          resource_pressure: "Resource pressure",
          resource_pressure_subtitle: "Process and machine health refreshed every five seconds.",
          refreshing: "Refreshing...",
          waiting_for_data: "Waiting for data...",
          latest_requests: "Latest requests",
          latest_requests_subtitle: "Newest 1000 requests, excluding the dashboard JSON polling endpoint.",
          table_time: "Time",
          table_method: "Method",
          table_status: "Status",
          table_path: "Path",
          table_client: "Client",
          table_latency: "Latency",
          table_bytes: "Bytes",
          top_routes: "Top routes",
          top_routes_subtitle: "Most frequently requested paths in this process lifetime.",
          client_footprint: "Client footprint",
          client_footprint_subtitle: "Unique client IPs and their request volume.",
          request_mix: "Request mix",
          request_mix_subtitle: "Method and status distribution across observed traffic.",
          method_distribution: "HTTP method distribution",
          status_distribution: "Response status distribution",
          summary_total_requests: "Total requests",
          summary_total_requests_note: "Last seen: {{value}}",
          summary_unique_routes: "Unique routes",
          summary_unique_routes_note: "Distinct URL paths observed",
          summary_unique_clients: "Unique clients",
          summary_unique_clients_note: "Distinct IP addresses observed",
          summary_transferred: "Transferred",
          summary_transferred_note: "Based on response Content-Length",
          summary_throughput: "Req throughput",
          summary_throughput_note: "Uptime: {{value}}",
          summary_pid: "PID",
          summary_pid_note: "Listening on {{value}}",
          process_cpu: "Process CPU",
          system_cpu: "System CPU",
          process_memory: "Process memory",
          system_memory: "System memory",
          no_history_yet: "No history yet.",
          no_traffic_yet: "No traffic yet.",
          no_requests_yet: "No requests observed yet.",
          no_route_traffic_yet: "No route traffic yet.",
          no_client_ips_yet: "No client IPs observed yet.",
          unknown_agent: "Unknown agent",
          requests_per_minute: "{{value}} req/min",
          duration_ms: "{{value}} ms",
          duration_s: "{{value}} s",
          captured_count: "{{count}} captured",
          refresh_updated: "Updated {{time}}",
          refresh_failed: "Refresh failed: {{message}}",
          load_avg: "Load avg: {{one}} / {{five}} / {{fifteen}}",
          virtual_memory: "Virtual memory: {{value}}",
          other_routes: "[other routes]",
          other_clients: "[other clients]"
        }},
        zh: {{
          page_title: "serve-here 统计面板",
          eyebrow: "实时运行观测",
          hero_title: "服务控制台",
          hero_description: "实时查看当前 serve-here 进程的请求流量、路由活跃度、客户端覆盖情况与资源压力。",
          theme_label: "主题",
          theme_dark: "深色",
          theme_light: "亮色",
          language_label: "语言",
          browse_files: "浏览文件",
          project_repo: "项目仓库",
          served_directory: "服务目录",
          bind_address: "监听地址",
          dashboard_started: "面板启动时间",
          resource_pressure: "资源压力",
          resource_pressure_subtitle: "每 5 秒刷新一次进程与机器健康状态。",
          refreshing: "刷新中...",
          waiting_for_data: "等待数据...",
          latest_requests: "最新请求",
          latest_requests_subtitle: "最近 1000 条请求，不包含统计面板 JSON 轮询接口。",
          table_time: "时间",
          table_method: "方法",
          table_status: "状态",
          table_path: "路径",
          table_client: "客户端",
          table_latency: "延迟",
          table_bytes: "字节数",
          top_routes: "热门路由",
          top_routes_subtitle: "当前进程生命周期内请求最频繁的路径。",
          client_footprint: "客户端分布",
          client_footprint_subtitle: "不同客户端 IP 及其请求量。",
          request_mix: "请求构成",
          request_mix_subtitle: "当前流量中的方法与状态码分布。",
          method_distribution: "HTTP 方法分布",
          status_distribution: "响应状态分布",
          summary_total_requests: "总请求数",
          summary_total_requests_note: "最近请求：{{value}}",
          summary_unique_routes: "唯一路由",
          summary_unique_routes_note: "已观测到的不同 URL 路径",
          summary_unique_clients: "唯一客户端",
          summary_unique_clients_note: "已观测到的不同 IP 地址",
          summary_transferred: "传输字节",
          summary_transferred_note: "基于响应 Content-Length",
          summary_throughput: "请求吞吐",
          summary_throughput_note: "运行时长：{{value}}",
          summary_pid: "进程 PID",
          summary_pid_note: "监听于 {{value}}",
          process_cpu: "进程 CPU",
          system_cpu: "系统 CPU",
          process_memory: "进程内存",
          system_memory: "系统内存",
          no_history_yet: "暂无历史数据。",
          no_traffic_yet: "暂无流量。",
          no_requests_yet: "尚未观测到请求。",
          no_route_traffic_yet: "暂无路由流量。",
          no_client_ips_yet: "暂无客户端 IP。",
          unknown_agent: "未知客户端",
          requests_per_minute: "{{value}} 次/分钟",
          duration_ms: "{{value}} 毫秒",
          duration_s: "{{value}} 秒",
          captured_count: "已记录 {{count}} 条",
          refresh_updated: "更新于 {{time}}",
          refresh_failed: "刷新失败：{{message}}",
          load_avg: "平均负载：{{one}} / {{five}} / {{fifteen}}",
          virtual_memory: "虚拟内存：{{value}}",
          other_routes: "【其他路由】",
          other_clients: "【其他客户端】"
        }}
      }};

      const root = document.documentElement;
      const ui = {{
        theme: root.dataset.theme || "dark",
        lang: root.dataset.lang || "en"
      }};
      let lastSnapshot = null;

      const startedAtEl = document.getElementById("startedAt");
      const refreshState = document.getElementById("refreshState");
      const requestCount = document.getElementById("requestCount");
      const themeButtons = Array.from(document.querySelectorAll("[data-theme-choice]"));
      const langButtons = Array.from(document.querySelectorAll("[data-lang-choice]"));

      function translate(key, vars = {{}}) {{
        const table = copy[ui.lang] || copy.en;
        let value = table[key] || copy.en[key] || key;
        for (const [name, replacement] of Object.entries(vars)) {{
          value = value.replaceAll(`{{${{name}}}}`, replacement);
        }}
        return value;
      }}

      function locale() {{
        return ui.lang === "zh" ? "zh-CN" : "en-US";
      }}

      function formatNumber(value) {{
        return new Intl.NumberFormat(locale()).format(value ?? 0);
      }}

      function formatBytes(bytes) {{
        if (!bytes) return "0 B";
        const units = ["B", "KB", "MB", "GB", "TB"];
        let value = bytes;
        let unit = 0;
        while (value >= 1024 && unit < units.length - 1) {{
          value /= 1024;
          unit += 1;
        }}
        return `${{value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)}} ${{units[unit]}}`;
      }}

      function formatRate(value) {{
        return translate("requests_per_minute", {{ value: value.toFixed(1) }});
      }}

      function formatDate(ts) {{
        if (!ts) return "-";
        return new Date(ts).toLocaleString(locale());
      }}

      function formatTime(ts) {{
        return new Date(ts).toLocaleTimeString(locale(), {{
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit"
        }});
      }}

      function escapeHtml(value) {{
        return String(value ?? "")
          .replaceAll("&", "&amp;")
          .replaceAll("<", "&lt;")
          .replaceAll(">", "&gt;")
          .replaceAll('"', "&quot;")
          .replaceAll("'", "&#39;");
      }}

      function formatDuration(ms) {{
        if (ms < 1000) {{
          return translate("duration_ms", {{ value: formatNumber(ms) }});
        }}
        return translate("duration_s", {{ value: (ms / 1000).toFixed(ms >= 10000 ? 0 : 1) }});
      }}

      function statusClass(status) {{
        if (status >= 500) return "status-5xx";
        if (status >= 400) return "status-4xx";
        if (status >= 300) return "status-3xx";
        return "status-2xx";
      }}

      function applyTheme() {{
        root.dataset.theme = ui.theme;
        localStorage.setItem("serve-here-theme", ui.theme);
        themeButtons.forEach(button => {{
          button.classList.toggle("active", button.dataset.themeChoice === ui.theme);
        }});
      }}

      function applyLanguage() {{
        root.dataset.lang = ui.lang;
        root.lang = ui.lang === "zh" ? "zh-CN" : "en";
        localStorage.setItem("serve-here-lang", ui.lang);
        document.title = translate("page_title");
        document.querySelectorAll("[data-i18n]").forEach(node => {{
          node.textContent = translate(node.dataset.i18n);
        }});
        langButtons.forEach(button => {{
          button.classList.toggle("active", button.dataset.langChoice === ui.lang);
        }});
        startedAtEl.textContent = formatDate(Number(startedAtEl.dataset.startedAtMs));
        if (lastSnapshot) {{
          render(lastSnapshot);
        }} else {{
          refreshState.textContent = translate("refreshing");
          requestCount.textContent = translate("captured_count", {{ count: "0" }});
        }}
      }}

      function renderSummaryCards(snapshot) {{
        const uniqueRoutesValue = `${{formatNumber(snapshot.overview.unique_routes)}}${{snapshot.overview.unique_routes_capped ? "+" : ""}}`;
        const uniqueClientsValue = `${{formatNumber(snapshot.overview.unique_clients)}}${{snapshot.overview.unique_clients_capped ? "+" : ""}}`;
        const cards = [
          [
            translate("summary_total_requests"),
            formatNumber(snapshot.overview.total_requests),
            translate("summary_total_requests_note", {{ value: formatDate(snapshot.overview.last_request_at_ms) }})
          ],
          [
            translate("summary_unique_routes"),
            uniqueRoutesValue,
            translate("summary_unique_routes_note")
          ],
          [
            translate("summary_unique_clients"),
            uniqueClientsValue,
            translate("summary_unique_clients_note")
          ],
          [
            translate("summary_transferred"),
            formatBytes(snapshot.overview.total_bytes_sent),
            translate("summary_transferred_note")
          ],
          [
            translate("summary_throughput"),
            formatRate(snapshot.overview.requests_per_minute),
            translate("summary_throughput_note", {{ value: formatDuration(snapshot.uptime_seconds * 1000) }})
          ],
          [
            translate("summary_pid"),
            formatNumber(snapshot.server.pid),
            translate("summary_pid_note", {{ value: `${{snapshot.server.host}}:${{snapshot.server.port}}` }})
          ]
        ];

        document.getElementById("summaryCards").innerHTML = cards
          .map(([label, value, note]) => `
            <article class="summary-card">
              <div class="summary-label">${{label}}</div>
              <div class="summary-value">${{value}}</div>
              <div class="summary-note">${{note}}</div>
            </article>
          `)
          .join("");
      }}

      function renderResourceCards(snapshot) {{
        const totalMemory = snapshot.resources.system_memory_total_bytes || 1;
        const processMemoryRatio = Math.min(100, (snapshot.resources.process_memory_bytes / totalMemory) * 100);
        const cards = [
          [translate("process_cpu"), `${{snapshot.resources.process_cpu_percent.toFixed(1)}}%`, snapshot.resources.process_cpu_percent],
          [translate("system_cpu"), `${{snapshot.resources.system_cpu_percent.toFixed(1)}}%`, snapshot.resources.system_cpu_percent],
          [translate("process_memory"), formatBytes(snapshot.resources.process_memory_bytes), processMemoryRatio],
          [translate("system_memory"), `${{formatBytes(snapshot.resources.system_memory_used_bytes)}} / ${{formatBytes(snapshot.resources.system_memory_total_bytes)}}`, snapshot.resources.system_memory_percent]
        ];

        document.getElementById("resourceCards").innerHTML = cards
          .map(([label, value, progress]) => `
            <article class="resource-card">
              <div class="resource-label">${{label}}</div>
              <div class="resource-value">${{value}}</div>
              <div class="resource-bar"><span style="width:${{Math.max(2, Math.min(100, progress)).toFixed(1)}}%"></span></div>
            </article>
          `)
          .join("");
      }}

      function renderLineChart(nodeId, points, accessor, stroke) {{
        const host = document.getElementById(nodeId);
        if (!points.length) {{
          host.innerHTML = `<div class="muted">${{translate("no_history_yet")}}</div>`;
          return;
        }}

        const width = 560;
        const height = 170;
        const padding = 12;
        const values = points.map(accessor);
        const max = Math.max(...values, 1);
        const min = Math.min(...values, 0);
        const span = Math.max(max - min, 1);

        const coords = values.map((value, index) => {{
          const x = padding + (index / Math.max(points.length - 1, 1)) * (width - padding * 2);
          const y = height - padding - ((value - min) / span) * (height - padding * 2);
          return `${{x.toFixed(2)}},${{y.toFixed(2)}}`;
        }});

        const area = [`${{padding}},${{height - padding}}`, ...coords, `${{width - padding}},${{height - padding}}`].join(" ");

        host.innerHTML = `
          <svg viewBox="0 0 ${{width}} ${{height}}" preserveAspectRatio="none" aria-hidden="true">
            <defs>
              <linearGradient id="${{nodeId}}Gradient" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stop-color="${{stroke}}" stop-opacity="0.34"></stop>
                <stop offset="100%" stop-color="${{stroke}}" stop-opacity="0"></stop>
              </linearGradient>
            </defs>
            <polyline fill="url(#${{nodeId}}Gradient)" stroke="none" points="${{area}}"></polyline>
            <polyline fill="none" stroke="${{stroke}}" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" points="${{coords.join(" ")}}"></polyline>
          </svg>
        `;
      }}

      function renderTopList(nodeId, items, emptyKey) {{
        const max = Math.max(...items.map(item => item.count), 1);
        document.getElementById(nodeId).innerHTML = items.length
          ? items.map(item => `
              <div class="list-row">
                <div class="list-label">
                  <strong title="${{escapeHtml(item.label)}}">${{escapeHtml(item.label)}}</strong>
                  <div class="meter"><span style="width:${{(item.count / max) * 100}}%"></span></div>
                </div>
                <div>${{formatNumber(item.count)}}</div>
              </div>
            `).join("")
          : `<div class="muted">${{translate(emptyKey)}}</div>`;
      }}

      function renderPills(nodeId, items) {{
        document.getElementById(nodeId).innerHTML = items.length
          ? items.map(item => item).join("")
          : `<span class="muted">${{translate("no_traffic_yet")}}</span>`;
      }}

      function renderRecentRequests(snapshot) {{
        requestCount.textContent = translate("captured_count", {{
          count: formatNumber(snapshot.recent_requests.length)
        }});

        document.getElementById("recentRequests").innerHTML = snapshot.recent_requests.length
          ? snapshot.recent_requests.map(item => `
              <tr>
                <td>${{formatDate(item.timestamp_ms)}}</td>
                <td><span class="method-badge">${{item.method}}</span></td>
                <td><span class="status-badge ${{statusClass(item.status)}}">${{item.status}}</span></td>
                <td>
                  <div>${{escapeHtml(item.path)}}</div>
                  <div class="muted">${{escapeHtml(item.user_agent || translate("unknown_agent"))}}</div>
                </td>
                <td>${{escapeHtml(item.client_ip || "-")}}</td>
                <td>${{formatDuration(item.duration_ms)}}</td>
                <td>${{item.response_bytes ? formatBytes(item.response_bytes) : "-"}}</td>
              </tr>
            `).join("")
          : `<tr><td colspan="7" class="muted">${{translate("no_requests_yet")}}</td></tr>`;
      }}

      function render(snapshot) {{
        renderSummaryCards(snapshot);
        renderResourceCards(snapshot);
        renderTopList("topRoutes", snapshot.top_routes, "no_route_traffic_yet");
        renderTopList("topClients", snapshot.top_clients, "no_client_ips_yet");
        renderPills(
          "methodPills",
          snapshot.methods.map(item => `<span class="pill"><strong>${{item.label}}</strong><span>${{formatNumber(item.count)}}</span></span>`)
        );
        renderPills(
          "statusPills",
          snapshot.statuses.map(item => `<span class="pill"><strong>${{item.status}}</strong><span>${{formatNumber(item.count)}}</span></span>`)
        );
        renderRecentRequests(snapshot);
        renderLineChart("cpuChart", snapshot.resources.history, item => item.process_cpu_percent, "#77c8ff");
        renderLineChart("memoryChart", snapshot.resources.history, item => item.process_memory_bytes, "#7c5cff");
        document.getElementById("cpuCaption").textContent = translate("load_avg", {{
          one: snapshot.resources.load_avg_one.toFixed(2),
          five: snapshot.resources.load_avg_five.toFixed(2),
          fifteen: snapshot.resources.load_avg_fifteen.toFixed(2)
        }});
        document.getElementById("memoryCaption").textContent = translate("virtual_memory", {{
          value: formatBytes(snapshot.resources.process_virtual_memory_bytes)
        }});
        refreshState.textContent = translate("refresh_updated", {{
          time: formatTime(snapshot.generated_at_ms)
        }});
      }}

      async function refresh() {{
        refreshState.textContent = translate("refreshing");

        try {{
          const response = await fetch("{}", {{ cache: "no-store" }});
          if (!response.ok) {{
            throw new Error(`Request failed with ${{response.status}}`);
          }}

          lastSnapshot = await response.json();
          render(lastSnapshot);
        }} catch (error) {{
          refreshState.textContent = translate("refresh_failed", {{ message: error.message }});
        }}
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

      applyTheme();
      applyLanguage();
      refresh();
      setInterval(refresh, 5000);
    </script>
  </body>
</html>"##,
        escape_html(root_dir),
        escape_html(&format_url_host(host)),
        port,
        started_at_ms,
        escape_attr(dashboard_data_path)
    )
}
