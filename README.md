# serve-here

Serve any local directory over HTTP with a single command.
用一条命令把任意本地目录通过 HTTP 暴露出去。

> **v2.0.0**: Rust backend, native npm distribution, polished file browser, and built-in live stats dashboard.
>
> **v2.0.0**：Rust 后端、原生 npm 分发、精致目录浏览页，以及内置实时统计面板。

## Screenshots | 截图

### Directory Browser | 目录浏览页

![Directory browser screenshot](docs/screenshots/listing-light.png)

### Stats Dashboard | 统计面板

![Stats dashboard screenshot](docs/screenshots/stats-light.png)

## Features | 功能特点

- Instant static hosting for the current or specified directory.
  立即托管当前目录或指定目录。
- Clean directory browser with filtering, metadata, compact layout, and live navigation.
  精致目录浏览页，支持筛选、元数据展示、紧凑布局和快速导航。
- Light and dark themes, with bilingual English/Chinese UI toggles.
  支持亮色/深色主题，并可在中英文界面之间切换。
- Automatic `index.html` support plus safe redirects for non-ASCII paths.
  自动识别 `index.html`，并安全处理中文等非 ASCII 路径跳转。
- Built-in `/stats` dashboard with request volume, route heat, client footprint, and system/process resource charts.
  内置 `/stats` 运行状态面板，可查看请求量、热门路由、客户端分布，以及系统/进程资源图表。
- Native binary distribution via npm optional platform packages.
  通过 npm 可选平台包分发原生二进制文件。
- Multi-platform support: macOS (Intel/Apple Silicon), Linux (x64/ARM64), Windows (x64).
  多平台支持：macOS（Intel / Apple Silicon）、Linux（x64 / ARM64）、Windows（x64）。

## Installation | 安装

```sh
npm install -g @chenpu17/serve-here
```

Or run ad-hoc:
或者临时执行：

```sh
npx @chenpu17/serve-here
```

## Usage | 使用方式

```sh
serve-here [options] [directory]
```

- `directory`: Directory to share; defaults to the current working directory.
  `directory`：要共享的目录，默认当前工作目录。
- `-d, --dir <path>`: Explicit directory override.
  `-d, --dir <path>`：显式指定共享目录。
- `-p, --port <number>`: Port to listen on (default `8080`).
  `-p, --port <number>`：监听端口，默认 `8080`。
- `-H, --host <address>`: Host/IP to bind (default `0.0.0.0`).
  `-H, --host <address>`：绑定主机或 IP，默认 `0.0.0.0`。
- `-D, --daemon`: Run as background daemon (Unix only).
  `-D, --daemon`：以守护进程运行（仅 Unix）。
- `--stop`: Stop a running daemon (use with `-p` to specify port).
  `--stop`：停止守护进程（可配合 `-p` 指定端口）。
- `--status`: Show status of running daemon(s).
  `--status`：查看守护进程状态。

After startup, the terminal prints accessible addresses. If the directory contains no `index.html`, the browser shows the built-in directory UI. Visit `/stats` for the runtime dashboard.
启动后终端会打印可访问地址。如果目录中没有 `index.html`，浏览器会显示内置目录 UI。访问 `/stats` 可打开运行状态面板。

## Development | 开发

Build and run locally:
本地构建与运行：

```sh
cd src-rust
cargo run -- /path/to/dir
```

Run Rust tests:
运行 Rust 测试：

```sh
cd src-rust
cargo test
```

Run E2E web UI tests:
运行 Web UI 端到端测试：

```sh
npx playwright test e2e/webui.spec.js --reporter=line
```

## Release Pipeline | 发布流水线

Tag-based release:
基于标签的发布：

```sh
git tag v2.0.0
git push origin v2.0.0
```

Current GitHub Actions behavior:
当前 GitHub Actions 行为：

- `CI` runs `cargo check`, `cargo clippy`, `cargo test`, and release builds for all supported targets.
  `CI` 会执行 `cargo check`、`cargo clippy`、`cargo test`，并构建所有支持的平台目标。
- `CD` triggers on `v*.*.*` tags, verifies tests again, builds platform binaries, publishes platform-specific npm packages, then publishes the main npm package.
  `CD` 在 `v*.*.*` 标签触发，重新执行测试，构建平台二进制，先发布平台 npm 包，再发布主 npm 包。
- The workflow is versioned by semver tag, so npm will retain multiple released versions.
  工作流按语义化版本标签发布，因此 npm 会保留多个已发布版本。
- The npm distribution is multi-platform, not multi-channel: it publishes one main package plus per-platform packages.
  npm 分发是多平台的，不是多发布通道的：会发布一个主包和多个平台子包。
- Re-runs are idempotent for already-published versions: the workflow now skips packages that already exist on npm.
  对已发布版本可安全重跑：工作流现在会跳过 npm 上已经存在的版本。

If you need prerelease channels such as `beta` or `next`, add npm dist-tag logic on top of the current semver workflow.
如果需要 `beta`、`next` 之类的预发布通道，需要在当前语义化版本工作流之上再增加 npm dist-tag 逻辑。
